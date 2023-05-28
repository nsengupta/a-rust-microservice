use std::sync::Mutex;

use crate::{sessions::SessionsOps, users::UsersOps};

use tonic::{Request, Response, Status};

use authentication::auth_server::Auth;
use authentication::{
    SignInRequest, SignInResponse, SignOutRequest, SignOutResponse, SignUpRequest, SignUpResponse,
    StatusCode,
};

pub mod authentication {
    tonic::include_proto!("authentication");
}

// Re-exporting
pub use authentication::auth_server::AuthServer;
pub use tonic::transport::Server;

pub struct AuthService {
    users_service: Box<Mutex<dyn UsersOps + Send + Sync>>,
    sessions_service: Box<Mutex<dyn SessionsOps + Send + Sync>>,
}

impl AuthService {
    pub fn new(
        users_service: Box<Mutex<dyn UsersOps + Send + Sync>>,
        sessions_service: Box<Mutex<dyn SessionsOps + Send + Sync>>,
    ) -> Self {
        Self {
            users_service,
            sessions_service,
        }
    }
}

#[tonic::async_trait]
impl Auth for AuthService {
    async fn sign_in(
        &self,
        request: Request<SignInRequest>,
    ) -> Result<Response<SignInResponse>, Status> {
        println!("Got a request: {:?}", request);

        let req = request.into_inner();


        // Get user's uuid from `users_service`. Panic if the lock is poisoned.
        let reply: SignInResponse = 
            if self.users_service.is_poisoned() 
                    { panic!("user service lock seems broken!") }
            else {
               let uuid = match self.users_service.lock() {
                                Ok(user) => user.get_user_uuid(req.username, req.password),
                                Err(x) => None
               };
               uuid

            }
            .map(|maybe_uuid| {

                let session = self
                .sessions_service
                .lock()
                .expect("session service lock seems broken!")
                .create_session(&maybe_uuid);

                (maybe_uuid, session)
            })
            .map_or_else(
                || {
                    SignInResponse {
                        status_code: 0,
                        user_uuid: "Not assigned".to_owned(),
                        session_token: "Not created".to_owned(),
                    }
                }, 
                |(maybe_uuid,session_id)| {
                    SignInResponse {
                        status_code: 1,
                        user_uuid: maybe_uuid.to_owned(),
                        session_token: session_id.to_owned(),   
                    }
                } 
            );


        // Match on `result`. If `result` is `None` return a SignInResponse with a the `status_code` set to `Failure`
        // and `user_uuid`/`session_token` set to empty strings.
        // let user_uuid: String = todo!();

        // let session_token: String = todo!(); // Create new session using `sessions_service`. Panic if the lock is poisoned.

        // let reply: SignInResponse = todo!(); // Create a `SignInResponse` with `status_code` set to `Success`

        Ok(Response::new(reply))
    }

    async fn sign_up(
        &self,
        request: Request<SignUpRequest>,
    ) -> Result<Response<SignUpResponse>, Status> {
        println!("Got a request: {:?}", request);

        let req = request.into_inner();

        let result: SignUpResponse = self
        .users_service
        .lock()
        .expect("user service lock seems broken!")
        .create_user(req.username, req.password)
        .map_or_else(
            |_| {
                SignUpResponse {
                    status_code: StatusCode::Failure.into()
                }
            },
            |v| {
                SignUpResponse {
                    status_code: StatusCode::Success.into()
                }
            }
        );

        Ok(Response::new(result))
        

       
    }

    async fn sign_out(
        &self,
        request: Request<SignOutRequest>,
    ) -> Result<Response<SignOutResponse>, Status> {
        println!("Got a request: {:?}", request);

        let req = request.into_inner();

        // TODO: Delete session using `sessions_service`.
        
        // Create `SignOutResponse` with `status_code` set to `Success`

        self
        .sessions_service
        .lock()
        .expect("user service lock seems broken, while signing out!")
        .delete_session(&req.session_token)
        ;

        let reply: SignOutResponse = SignOutResponse {
            status_code: StatusCode::Success.into()
        };

        Ok(Response::new(reply))
    }

}

#[cfg(test)]
mod tests {
    use crate::{users::UsersImpl, sessions::SessionsImpl};

    use super::*;

    #[tokio::test]
    async fn sign_in_should_fail_if_user_not_found() {
        let users_service = Box::new(Mutex::new(UsersImpl::default()));
        let sessions_service = Box::new(Mutex::new(SessionsImpl::default()));

        let auth_service = AuthService::new(users_service, sessions_service);

        let request = tonic::Request::new(SignInRequest {
            username: "123456".to_owned(),
            password: "654321".to_owned(),
        });

        let result = auth_service.sign_in(request).await.unwrap().into_inner();

        assert_eq!(result.status_code, StatusCode::Failure.into());
        assert_eq!(result.user_uuid.is_empty(), true);
        assert_eq!(result.session_token.is_empty(), true);
    }

    #[tokio::test]
    async fn sign_in_should_fail_if_incorrect_password() {
        let mut users_service = UsersImpl::default();

        let _ = users_service.create_user("123456".to_owned(), "654321".to_owned());

        let users_service = Box::new(Mutex::new(users_service));
        let sessions_service = Box::new(Mutex::new(SessionsImpl::default()));

        let auth_service = AuthService::new(users_service, sessions_service);

        let request = tonic::Request::new(SignInRequest {
            username: "123456".to_owned(),
            password: "wrong password".to_owned(),
        });

        let result = auth_service.sign_in(request).await.unwrap().into_inner();

        assert_eq!(result.status_code, StatusCode::Failure.into());
        assert_eq!(result.user_uuid.is_empty(), true);
        assert_eq!(result.session_token.is_empty(), true);
    }

    #[tokio::test]
    async fn sign_in_should_succeed() {
        let mut users_service = UsersImpl::default();

        let _ = users_service.create_user("123456".to_owned(), "654321".to_owned());

        let users_service = Box::new(Mutex::new(users_service));
        let sessions_service = Box::new(Mutex::new(SessionsImpl::default()));

        let auth_service = AuthService::new(users_service, sessions_service);

        let request = tonic::Request::new(SignInRequest {
            username: "123456".to_owned(),
            password: "654321".to_owned(),
        });

        let result = auth_service.sign_in(request).await.unwrap().into_inner();

        assert_eq!(result.status_code, StatusCode::Success.into());
        assert_eq!(result.user_uuid.is_empty(), false);
        assert_eq!(result.session_token.is_empty(), false);
    }

    #[tokio::test]
    async fn sign_up_should_fail_if_username_exists() {
        let mut users_service = UsersImpl::default();

        let _ = users_service.create_user("123456".to_owned(), "654321".to_owned());

        let users_service = Box::new(Mutex::new(users_service));
        let sessions_service = Box::new(Mutex::new(SessionsImpl::default()));

        let auth_service = AuthService::new(users_service, sessions_service);

        let request = tonic::Request::new(SignUpRequest {
            username: "123456".to_owned(),
            password: "654321".to_owned(),
        });

        let result = auth_service.sign_up(request).await.unwrap();

        assert_eq!(result.into_inner().status_code, StatusCode::Failure.into());
    }

    #[tokio::test]
    async fn sign_up_should_succeed() {
        let users_service = Box::new(Mutex::new(UsersImpl::default()));
        let sessions_service = Box::new(Mutex::new(SessionsImpl::default()));

        let auth_service = AuthService::new(users_service, sessions_service);

        let request = tonic::Request::new(SignUpRequest {
            username: "123456".to_owned(),
            password: "654321".to_owned(),
        });

        let result = auth_service.sign_up(request).await.unwrap();

        assert_eq!(result.into_inner().status_code, StatusCode::Success.into());
    }

    #[tokio::test]
    async fn sign_out_should_succeed() {
        let users_service = Box::new(Mutex::new(UsersImpl::default()));
        let sessions_service = Box::new(Mutex::new(SessionsImpl::default()));

        let auth_service = AuthService::new(users_service, sessions_service);

        let request = tonic::Request::new(SignOutRequest {
            session_token: "".to_owned()
        });

        let result = auth_service.sign_out(request).await.unwrap();

        assert_eq!(result.into_inner().status_code, StatusCode::Success.into());
    }
}
