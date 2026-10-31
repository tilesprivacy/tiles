// use anyhow::Result;
// use std::path::PathBuf;
// use tempfile::TempDir;
// use tiles::utils::config::ConfigProvider;

// #[derive(Default, Debug)]
// struct MockProvider {
//     tmpdir: TempDir,
// }

// impl ConfigProvider for MockProvider {
//     fn get_config_dir(&self) -> Result<PathBuf> {
//         Ok(self.tmpdir.path().to_path_buf())
//     }
//     fn get_or_create_config_dir(&self) -> Result<PathBuf> {
//         Ok(self.tmpdir.path().to_path_buf())
//     }
//     fn get_data_dir(&self) -> Result<PathBuf> {
//         Ok(self.tmpdir.path().to_path_buf())
//     }
//     fn get_or_create_data_dir(&self) -> Result<PathBuf> {
//         Ok(self.tmpdir.path().to_path_buf())
//     }
//     fn get_user_data_dir(&self) -> Result<PathBuf> {
//         Ok(self.tmpdir.path().to_path_buf())
//     }
//     fn get_lib_dir(&self) -> Result<PathBuf> {
//         Ok(self.tmpdir.path().to_path_buf())
//     }
//     fn get_user_bin_dir(&self) -> Result<PathBuf> {
//         Ok(self.tmpdir.path().to_path_buf())
//     }
//     fn get_user_bin_path(&self) -> Result<PathBuf> {
//         Ok(self.tmpdir.path().to_path_buf())
//     }
// }

// #[test]
// fn test_get_memory_path() -> Result<()> {
//     let _tmp = tempdir()?;
//     todo!()
// }

// #[test]
// fn set_and_get_memory_path_with_mock_provider() -> Result<()> {
//     let tmp = tempdir()?;
//     let config_dir = tmp.path().join("config");
//     let mem_dir = tmp.path().join("memdir");
//     fs::create_dir_all(&config_dir)?;
//     fs::create_dir_all(&mem_dir)?;

//     let provider = MockProvider {
//         base: tmp.path().to_path_buf(),
//     };

//     // set
//     let path_str = mem_dir.to_str().unwrap();
//     let res = set_memory_path_with_provider(&provider, path_str)?;
//     assert!(res.contains("Memory path set successfully"));

//     // verify file content
//     let stored = fs::read_to_string(config_dir.join(".memory_path"))?;
//     assert_eq!(stored, path_str);

//     // get
//     let got = get_memory_path_with_provider(&provider)?;
//     assert_eq!(got, path_str);

//     Ok(())
// }

// #[test]
// fn default_memory_path_from_provider() -> Result<()> {
//     let tmp = tempdir()?;
//     let provider = MockProvider {
//         base: tmp.path().to_path_buf(),
//     };
//     let default = get_default_memory_path_with_provider(&provider)?;
//     assert!(default.ends_with("data/memory") || default.ends_with("data\\memory"));
//     Ok(())
// }
