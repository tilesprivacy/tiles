//! APIs for user account related management

use crate::{
    core::account::local::{
        RootUser, create_root_account, get_root_user_details, save_root_account,
    },
    daemon::{ApiResponse, AppError, AppState},
    utils::config::{ConfigProvider, DefaultProvider, get_or_create_config},
};
use axum::{
    Json, Router,
    response::IntoResponse,
    routing::{get, post},
};
use serde::Deserialize;
use std::sync::Arc;

#[derive(Deserialize)]
struct CreateAccount {
    nickname: String,
}
/// Routers for account apis
///
/// These are to be merged with the main router in daemon/mod.rs
pub fn account_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/v1/tilekit/account/status", get(status))
        .route("/v1/tilekit/account/create", post(create_account))
}

async fn status() -> Result<impl IntoResponse, AppError> {
    get_account_status(DefaultProvider)
}

fn get_account_status(provider: impl ConfigProvider) -> Result<impl IntoResponse, AppError> {
    let config =
        get_or_create_config(provider).map_err(|e| AppError::InternalServerError(e.to_string()))?;
    let root_user_details =
        get_root_user_details(&config).map_err(|e| AppError::BadRequest(e.to_string()))?;

    if root_user_details.id.is_empty() {
        Err(AppError::NotFound(
            "Not local account found, please create one".to_string(),
        ))
    } else {
        Ok(ApiResponse::success(root_user_details))
    }
}

async fn create_account(Json(payload): Json<CreateAccount>) -> Result<impl IntoResponse, AppError> {
    do_create_account(DefaultProvider, payload).await
}

async fn do_create_account(
    provider: impl ConfigProvider,
    payload: CreateAccount,
) -> Result<impl IntoResponse, AppError> {
    println!("{:?}", provider.get_config_dir().unwrap());
    let config =
        get_or_create_config(provider).map_err(|e| AppError::InternalServerError(e.to_string()))?;

    let root_user_details =
        get_root_user_details(&config).map_err(|e| AppError::BadRequest(e.to_string()))?;

    println!("{:?}", root_user_details);
    if !root_user_details.id.is_empty() {
        let err_msg = format!("Local Identity exists with id: {}", root_user_details.id);
        Err(AppError::AlreadyExists(err_msg))
    } else {
        let root_user_config = RootUser::new(
            &create_root_account(&config, Some(payload.nickname))
                .await
                .map_err(|e| AppError::InternalServerError(e.to_string()))?,
        )
        .map_err(|e| AppError::InternalServerError(e.to_string()))?;

        save_root_account(config, &root_user_config.to_table())
            .map_err(|e| AppError::InternalServerError(e.to_string()))?;
        Ok(ApiResponse::success(root_user_config))
    }
}

#[cfg(test)]
pub mod tests {

    use crate::{
        daemon::account::{CreateAccount, do_create_account, get_account_status},
        utils::config::ConfigProvider,
    };
    use anyhow::Result;
    use axum::response::IntoResponse;
    use http_body_util::BodyExt;
    use reqwest::StatusCode;
    use std::path::PathBuf;
    use tempfile::tempdir;

    #[derive(Debug, Clone)]
    pub struct MockProvider {
        pub tmp_path: PathBuf,
    }

    impl ConfigProvider for MockProvider {
        fn get_config_dir(&self) -> Result<PathBuf> {
            Ok(self.tmp_path.clone())
        }
        fn get_or_create_config_dir(&self) -> Result<PathBuf> {
            Ok(self.tmp_path.clone())
        }
        fn get_data_dir(&self) -> Result<PathBuf> {
            Ok(self.tmp_path.clone())
        }
        fn get_or_create_data_dir(&self) -> Result<PathBuf> {
            Ok(self.tmp_path.clone())
        }
        fn get_user_data_dir(&self) -> Result<PathBuf> {
            Ok(self.tmp_path.clone())
        }
        fn get_lib_dir(&self) -> Result<PathBuf> {
            Ok(self.tmp_path.clone())
        }
        fn get_user_bin_dir(&self) -> Result<PathBuf> {
            Ok(self.tmp_path.clone())
        }
        fn get_user_bin_path(&self) -> Result<PathBuf> {
            Ok(self.tmp_path.clone())
        }
    }
    fn use_sample_keyring_store() -> Result<()> {
        keyring_core::set_default_store(keyring_core::sample::Store::new()?);
        Ok(())
    }
    #[test]
    fn test_account_status() {
        let res = get_account_status(MockProvider {
            tmp_path: tempdir().unwrap().path().to_path_buf(),
        });

        assert!(res.is_err());

        if let Err(err) = res {
            let response = err.into_response();
            assert_eq!(response.status(), StatusCode::NOT_FOUND)
        }
    }
    #[tokio::test]
    async fn test_create_account() {
        use_sample_keyring_store().unwrap();
        let tmp_dir = tempdir().unwrap();
        let mock_provider = MockProvider {
            tmp_path: tmp_dir.path().to_path_buf(),
        };

        let res = do_create_account(
            mock_provider.clone(),
            CreateAccount {
                nickname: "madclaws".to_owned(),
            },
        )
        .await;

        assert!(res.is_ok());

        let response = res.into_response();
        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap();
        let b_bytes = body.to_bytes();
        assert!(
            String::from_utf8(b_bytes.to_vec())
                .unwrap()
                .contains("madclaws")
        );

        // For this test to work we need to refactor a lot, since some other
        // fns are using DefaultProvider
        // Fails, as the account exist already
        // let res = do_create_account(
        //     mock_provider,
        //     CreateAccount {
        //         nickname: "madclaws".to_owned(),
        //     },
        // )
        // .await;

        // assert!(res.is_ok());
        // let response = res.into_response();
        // // assert_eq!(response.status(), StatusCode::CONFLICT);
        // let body = response.into_body().collect().await.unwrap();
        // let b_bytes = body.to_bytes();
        // println!("{:?}", b_bytes);
    }
}
