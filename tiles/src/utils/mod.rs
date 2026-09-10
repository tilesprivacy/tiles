use std::{
    fs,
    path::{Component, Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

pub mod config;
pub mod crypto;
pub mod hf_model_downloader;
pub mod installer;
pub mod lexicons;
pub mod uninstaller;
pub fn get_unix_time_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time went backwards")
        .as_millis() as u64
}

pub fn test_logger() {
    let _ = env_logger::builder().is_test(true).try_init();
}

/// Copies a directory tree. Symlinks are kept only when they resolve inside
/// the tree being copied, so the copy stays self-contained; anything pointing
/// outside is refused. npm trees rely on this for `node_modules/.bin`.
pub fn copy_recursive(src: &Path, dest: &Path) -> anyhow::Result<()> {
    copy_tree(src, dest, src)
}

fn copy_tree(src: &Path, dest: &Path, root: &Path) -> anyhow::Result<()> {
    fs::create_dir_all(dest)?;

    let ls = fs::read_dir(src)?;

    for dir_result in ls {
        let dir_path = dir_result?;
        let file_type = dir_path.file_type()?;
        let filename = dir_path.file_name();
        let src_path = dir_path.path();
        let dest_path = PathBuf::new().join(dest).join(filename);
        if file_type.is_symlink() {
            let target = fs::read_link(&src_path)?;
            let link_dir = src_path.parent().unwrap_or(root);
            if !symlink_stays_within(root, link_dir, &target) {
                anyhow::bail!(
                    "Refusing to copy symlink entry {:?}: target {:?} escapes {:?}",
                    src_path,
                    target,
                    root
                );
            }
            // Recreate rather than dereference, so the tree keeps its shape
            // and we never duplicate large files.
            if dest_path.symlink_metadata().is_ok() {
                fs::remove_file(&dest_path)?;
            }
            std::os::unix::fs::symlink(&target, &dest_path)?;
        } else if file_type.is_dir() {
            copy_tree(&src_path, &dest_path, root)?
        } else {
            fs::copy(&src_path, &dest_path)?;
        }
    }

    Ok(())
}

/// Resolves a symlink target lexically and reports whether it stays under
/// `root`. Lexical, not `canonicalize`, so broken links are still judged.
fn symlink_stays_within(root: &Path, link_dir: &Path, target: &Path) -> bool {
    if target.is_absolute() {
        return false;
    }
    normalize_lexical(&link_dir.join(target)).starts_with(normalize_lexical(root))
}

fn normalize_lexical(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for part in path.components() {
        match part {
            Component::CurDir => (),
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{normalize_lexical, symlink_stays_within};
    use std::path::Path;

    #[test]
    fn symlink_inside_the_tree_is_allowed() {
        let root = Path::new("/plugins/demo");
        // npm's node_modules/.bin/foo -> ../foo/cli.js
        assert!(symlink_stays_within(
            root,
            Path::new("/plugins/demo/node_modules/.bin"),
            Path::new("../foo/cli.js"),
        ));
        assert!(symlink_stays_within(
            root,
            Path::new("/plugins/demo"),
            Path::new("./skills/a"),
        ));
    }

    #[test]
    fn symlink_escaping_the_tree_is_refused() {
        let root = Path::new("/plugins/demo");
        assert!(!symlink_stays_within(
            root,
            Path::new("/plugins/demo"),
            Path::new("../other/secret"),
        ));
        assert!(!symlink_stays_within(
            root,
            Path::new("/plugins/demo/node_modules/.bin"),
            Path::new("../../../../etc/passwd"),
        ));
        // absolute targets are never portable inside a package
        assert!(!symlink_stays_within(
            root,
            Path::new("/plugins/demo"),
            Path::new("/etc/passwd"),
        ));
    }

    #[test]
    fn normalize_lexical_resolves_dot_segments() {
        assert_eq!(
            normalize_lexical(Path::new("/a/b/../c/./d")),
            Path::new("/a/c/d")
        );
        // cannot climb above the filesystem root
        assert_eq!(normalize_lexical(Path::new("/../../x")), Path::new("/x"));
    }

    use std::fs;
    use tempfile::tempdir;

    use super::*;

    #[test]
    fn test_valid_copy() {
        let tmp = tempdir().expect("created tmp dir");

        let src = tmp.path().join("source");
        let dest = tmp.path().join("dest");

        fs::create_dir_all(src.join("skills").join("flamez")).unwrap();

        fs::write(
            src.join("skills").join("flamez").join("SKILLS.md"),
            "skill".as_bytes(),
        )
        .unwrap();

        copy_recursive(&src, &dest).unwrap();

        assert!(
            dest.join("skills")
                .join("flamez")
                .join("SKILLS.md")
                .exists()
        )
    }

    #[test]
    fn test_copy_keeps_internal_symlinks() {
        let tmp = tempdir().expect("created tmp dir");
        let src = tmp.path().join("source");
        let dest = tmp.path().join("dest");

        // mimic an npm tree: node_modules/.bin/dep -> ../dep/cli.js
        fs::create_dir_all(src.join("node_modules").join("dep")).unwrap();
        fs::create_dir_all(src.join("node_modules").join(".bin")).unwrap();
        fs::write(src.join("node_modules").join("dep").join("cli.js"), b"cli").unwrap();
        std::os::unix::fs::symlink("../dep/cli.js", src.join("node_modules/.bin/dep")).unwrap();

        copy_recursive(&src, &dest).unwrap();

        let link = dest.join("node_modules/.bin/dep");
        assert!(link.symlink_metadata().unwrap().file_type().is_symlink());
        // still a symlink, and it still points at real content
        assert_eq!(fs::read_to_string(&link).unwrap(), "cli");
    }

    #[test]
    fn test_copy_refuses_escaping_symlink() {
        let tmp = tempdir().expect("created tmp dir");
        let src = tmp.path().join("source");
        let dest = tmp.path().join("dest");

        fs::create_dir_all(&src).unwrap();
        fs::write(tmp.path().join("outside.txt"), b"secret").unwrap();
        std::os::unix::fs::symlink("../outside.txt", src.join("leak")).unwrap();

        let err = copy_recursive(&src, &dest).unwrap_err().to_string();
        assert!(err.contains("escapes"), "{}", err);
    }
}
