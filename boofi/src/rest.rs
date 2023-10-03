use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderValue, Method, Request, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::{Json, middleware, RequestExt, Router};
use axum::body::{Body, HttpBody};
use axum::middleware::Next;
use axum::routing::{any, get, post};
use base64::Engine;
use hyper::server::conn::{AddrIncoming, Http};
use rustls::{Certificate, PrivateKey, ServerConfig};
use rustls_pemfile::{certs, pkcs8_private_keys};
use serde::{Deserialize, Serialize};
use serde_json::{to_value, Value};
use tokio::net::TcpListener;
use crate::controller::Controller;
use crate::error::{Erro, Resul};
use crate::apps::{AppBuilders, AppHelp};
use crate::files::{FileHelp};
use tokio::sync::Mutex;
use tokio_rustls::TlsAcceptor;
use tower::MakeService;
use crate::apps::ls::{LsEntry, LsInput, LsApp};
use futures_util::future::poll_fn;
use hyper::server::accept::Accept;
use tokio::task::JoinHandle;
use crate::system::{Credential, System};

type SharedController = Arc<Mutex<Controller>>;

/// Used for authentication
#[derive(Debug)]
struct UsernamePassword {
    username: String,
    password: String,
}

impl From<&UsernamePassword> for Credential {
    fn from(value: &UsernamePassword) -> Self {
        Self::new(value.username.as_str(), value.password.as_str())
    }
}

/// Used to return the bearer token
#[derive(Debug, Serialize, Deserialize)]
struct TokenResult {
    token: String,
}

/// url query used in app context
#[derive(Debug, Deserialize)]
struct AppQuery {
    r#async: Option<bool>,
}

/// The request body for each app
#[derive(Debug, Serialize, Deserialize)]
struct AppsBodyApp {
    name: String,
    input: Value,
}

/// url query in file context
#[derive(Debug, Deserialize)]
struct FileQuery {
    name: Option<String>,
}

/// used in directory list context
#[derive(Debug, Serialize)]
struct DirItemExtended {
    info: DirItem,
    managed_by: Vec<String>,
}

/// Authentication middleware
/// all requests are pre processed within this method
async fn auth<B>(
    State(controller): State<SharedController>,
    mut request: Request<B>,
    next: Next<B>,
) -> Resul<Response> {
    if let Some(auth) = request.headers().get("authorization") {
        log::trace!("[AUTH] processing");
        let (typ, value) = auth.to_str()?.split_once(' ').ok_or(Erro::RestAuthMissing)?;

        let (username, password) = match typ {
            "Basic" | "basic" => {
                log::trace!("[AUTH][BASIC]");
                let decoded = base64::engine::general_purpose::STANDARD.decode(value).map(String::from_utf8)??;
                decoded.split_once(':').map(|(u, p)| (u.to_string(), p.to_string()))
                    .unwrap_or((decoded.to_string(), Default::default())) // no password provided, assume empty
            }
            "Bearer" | "bearer" => {
                log::trace!("[AUTH][BEARER]");
                controller.lock().await.auth_mut().get(value).map(|a| {
                    request.extensions_mut().insert(TokenResult {
                        token: a.token().into(),
                    });

                    (a.username().to_string(), a.password().to_string())
                })?
            }
            _ => return Err(Erro::RestAuthInvalid)
        };

        log::debug!("[AUTH] processed");
        request.extensions_mut().insert(UsernamePassword {
            username,
            password,
        });

        Ok(next.run(request).await)
    } else {
        log::debug!("[BASIC_AUTH] sending authentication request");

        let response = next.run(request).await;

        log::trace!("[BASIC_AUTH] set header");
        Ok(Response::builder().header("WWW-Authenticate",
                                      HeaderValue::from_str(r#"Basic realm="rest api""#)?)
            .status(StatusCode::UNAUTHORIZED).body(response.into_body())?)
    }
}

pub(crate) type ServicesConfig = HashMap<String, Router>;

/// REST API
pub(crate) struct Rest {
    address: SocketAddr,
}

impl Rest {
    pub(crate) fn new(address: SocketAddr) -> Self {
        Self {
            address,
        }
    }

    /// Creates a new router with the given configuration
    fn router(services: ServicesConfig) -> Router {
        let mut router = Router::new();

        for (mut name, service) in services {
            name.insert(0, '/');
            router = router.nest(&name, service);
            log::trace!("[START] service {} configured", name);
        }
        router
    }

    /// Starts all services
    pub(crate) async fn start(&self, services: ServicesConfig) -> Resul<()> {
        let app = Self::router(services);
        log::debug!("[START] starting server");

        let server = axum::Server::bind(&self.address)
            .serve(app.into_make_service());
        server.await.map_err(Into::into)
    }

    /// Starts all services but with https
    pub(crate) async fn ssl(&self, services: ServicesConfig, private_key: &str, certificate: &str) -> Resul<()> {
        let key: PrivateKey = PrivateKey(pkcs8_private_keys(&mut private_key.as_bytes())?.remove(0));
        let certs: Vec<Certificate> = certs(&mut certificate.as_bytes())?
            .into_iter()
            .map(Certificate)
            .collect();

        log::debug!("[REST SSL] prepared");

        let config = ServerConfig::builder()
            .with_safe_defaults()
            .with_no_client_auth()
            .with_single_cert(certs, key)?;

        log::debug!("[REST SSL] configured");

        let arc_config = Arc::new(config);
        let acceptor = TlsAcceptor::from(arc_config);

        let mut listener = AddrIncoming::from_listener(
            TcpListener::bind(self.address).await?)?;

        let protocol = Arc::new(Http::new());

        let mut app = Self::router(services).into_make_service();
        log::debug!("[REST SSL] router configured");

        loop {
            match poll_fn(|cx| Pin::new(&mut listener).poll_accept(cx)).await {
                None => {}
                Some(result) => {
                    let stream = result?;

                    log::trace!("[REST SSL] connection accepted");

                    let acceptor = acceptor.clone();
                    let protocol = protocol.clone();

                    let svc = MakeService::make_service(&mut app, &stream);

                    let _a: JoinHandle<Resul<()>> = tokio::spawn(async move {
                        match acceptor.accept(stream).await {
                            Ok(stream) => {
                                log::trace!("[REST SSL] serve connection");
                                let _ = protocol.serve_connection(stream, svc.await?).await;
                            }
                            Err(e) => {
                                log::error!("[REST SSL] {:?}", e);
                            }
                        }
                        Ok(())
                    });
                }
            }
        }
    }

    /// Creates all routes with their handlers
    fn routes() -> Router<SharedController> {
        Router::new()
            .route("/token", any(Self::token_get_delete))
            .route("/tasks", get(Self::tasks_get))
            .route("/tasks/:id", get(Self::tasks_get))
            .route("/apps", get(Self::apps_help))
            .route("/apps", post(Self::apps_post))
            .route("/apps/:name", post(Self::app_post))
            .route("/files", get(Self::files_help))
            .route("/files/", get(Self::files_get_post_delete))
            .route("/files/*key", any(Self::files_get_post_delete))
    }

    /// New single service with its own controller
    pub(crate) async fn new_service(&self, controller: Controller) -> Router<()> {
        let shared_controller = Arc::new(Mutex::new(controller));

        log::trace!("[NEW SERVICE] configure routes");

        Self::routes()
            .with_state(shared_controller.clone())
            .layer(middleware::from_fn_with_state(shared_controller, auth))
    }

    async fn token_get_delete(State(controller): State<SharedController>, request: Request<Body>) -> Resul<Response> {
        match *request.method() {
            Method::GET => {
                let user_password: &UsernamePassword = request.extensions().get().ok_or(Erro::RestAuthMissing)?;

                log::debug!("[TOKEN GET] verify credential");
                let mut ctrl = controller.lock().await;
                let system_manager = ctrl.system_manager_mut();
                let system = system_manager.system_credential(user_password.into()).await?;
                system.verify_credential().await?;
                log::debug!("[TOKEN GET] credential verified");

                Ok(Json(TokenResult {
                    token: ctrl.auth_mut().insert_or_replace(user_password.username.clone(),
                                                             user_password.password.clone())
                }).into_response())
            }
            Method::DELETE => {
                let mut ctrl = controller.lock().await;
                let token: &TokenResult = request.extensions()
                    .get()
                    .ok_or(Erro::RestAuthMissing)?;

                Ok(if ctrl.auth_mut().delete(&token.token) {
                    log::debug!("[TOKEN DELETE] token deleted");
                    StatusCode::ACCEPTED
                } else {
                    log::debug!("[TOKEN DELETE] token does not exist");
                    StatusCode::OK
                }.into_response())
            }
            _ => Err(Erro::HttpMethodNotAllowed(request.method().clone()))
        }
    }

    async fn apps_help(State(controller): State<SharedController>,
                       request: Request<Body>) -> Resul<Response> {
        log::trace!("[APPS HELP] getting authentication");
        let user_password: &UsernamePassword = request.extensions()
            .get()
            .ok_or(Erro::RestAuthMissing)?;

        let os = {
            let mut ctrl = controller.lock().await;
            let system_manager = ctrl.system_manager_mut();
            let system = system_manager.system_credential(user_password.into()).await?;

            log::debug!("[APPS HELP] sending help");
            system.os()?.clone()
        };

        Ok(Json(controller.lock().await.apps().iter().map(|app| app.help(&os)).collect::<Vec<AppHelp>>()).into_response())
    }

    async fn tasks_get(id: Option<Path<usize>>, State(controller): State<SharedController>, request: Request<Body>) -> Resul<Response> {
        let user_password: &UsernamePassword = request.extensions().get().ok_or(Erro::RestAuthMissing)?;
        let mut ctrl = controller.lock().await;
        let system_manager = ctrl.system_manager_mut();
        let system = system_manager.system_credential(user_password.into()).await?;
        system.verify_credential().await?;

        let task_ctrl = ctrl.task_controller();

        if let Some(i) = id {
            log::trace!("[TASKS GET] searching for task {}", *i);
            if let Some(task) = task_ctrl.tasks().lock().await.iter().find(|j| j.id() == *i) {
                Ok(Json(task).into_response())
            } else {
                Err(Erro::TaskNotFound)
            }
        } else {
            log::error!("[TASKS GET] no task id provided");
            Ok(Json(task_ctrl.tasks().lock().await.iter().map(|task| to_value(task)
                .map_err(Into::into))
                .collect::<Result<Vec<Value>, serde_json::Error>>()?).into_response())
        }
    }

    async fn apps_post(
        Query(query): Query<AppQuery>,
        State(controller): State<SharedController>,
        mut request: Request<Body>) -> Resul<Response> {
        log::trace!("[APPS POST] processing body request");
        let apps = serde_json::from_slice::<Vec<AppsBodyApp>>(&request.body_mut().data().await.ok_or(Erro::AppBodyMissing)??)?;
        let user_password: &UsernamePassword = request.extensions().get().ok_or(Erro::RestAuthMissing)?;

        // find apps
        let mut inputs_and_builders: Vec<(AppsBodyApp, AppBuilders)> = vec![];

        let os = {
            let mut ctrl = controller.lock().await;
            let system_manager = ctrl.system_manager_mut();
            system_manager.system_credential(user_password.into()).await?.os()?.clone()
        };

        log::debug!("[APPS POST] checking apps {} compatibility", apps.iter().map(|a| a.name.clone()).collect::<Vec<String>>().join(","));
        for app_body in apps {
            if let Some(app_builder) = controller.lock().await.app(&app_body.name) {
                if app_builder.compatible(&os) {
                    inputs_and_builders.push((app_body, app_builder.clone()));
                } else {
                    log::error!("[APPS POST] app {} incompatible", app_builder.name());
                    return Err(Erro::AppIncompatible);
                }
            } else {
                log::error!("[APPS POST] app {} not found", app_body.name);
                return Err(Erro::AppNotFound);
            }
        }

        let mut ctrl = controller.lock().await;
        let system = ctrl.system_manager_mut().system_credential(user_password.into()).await?.clone();

        // run apps (a)sync
        let mut results = vec![];
        for (app_body, mut managed_app) in inputs_and_builders {
            if query.r#async == Some(true) {
                log::debug!("[APPS POST] running app {} asynchronous", app_body.name);

                results.push(ctrl.task_controller_mut()
                    .new_task(managed_app, app_body.input, system.clone()).await?);
            } else {
                log::debug!("[APPS POST] running app {}", app_body.name);
                results.push(to_value(managed_app.run(app_body.input, &system).await?)?);
            }
        }

        Ok(Json(results).into_response())
    }

    async fn app_post(
        name: Path<String>,
        Query(query): Query<AppQuery>,
        State(controller): State<SharedController>,
        mut request: Request<Body>) -> Resul<Response> {
        log::trace!("[APP POST] processing body request");
        let value = serde_json::from_slice::<Value>(&request.body_mut().data().await.ok_or(Erro::AppBodyMissing)??)?;
        let user_password: &UsernamePassword = request.extensions().get().ok_or(Erro::RestAuthMissing)?;

        let (os, system) = {
            let mut ctrl = controller.lock().await;
            let system_manager = ctrl.system_manager_mut();
            let system = system_manager.system_credential(user_password.into()).await?.clone();
            (system.os()?.clone(), system)
        };

        let mut ctrl = controller.lock().await;
        if let Some(app_builder) = ctrl.app_mut(name.0.as_str()) {
            if !app_builder.compatible(&os) {
                log::error!("[APP POST] app incompatible");
                return Err(Erro::AppIncompatible);
            }

            if query.r#async == Some(true) {
                log::debug!("[APP POST] running app asynchronous");
                let app = app_builder.clone();
                return Ok(Json(ctrl.task_controller_mut().new_task(app, value, system).await?).into_response());
            } else {
                log::debug!("[APP POST] running app");
                return Ok(Json(app_builder.run(value, &system).await?).into_response());
            }
        }
        log::error!("[APP POST] no app found");

        Err(Erro::AppNotFound)
    }

    async fn files_help(State(controller): State<SharedController>) -> Resul<Response> {
        log::debug!("[FILES HELP] sending help");
        let ctrl = controller.lock().await;
        Ok(Json(ctrl.file_builders().iter().map(|file| file.help()).collect::<Vec<FileHelp>>()).into_response())
    }

    async fn files_get_post_delete(key: Option<Path<String>>,
                                   query: Query<FileQuery>,
                                   State(controller): State<SharedController>,
                                   request: Request<Body>) -> Resul<Response> {
        let p = format!("/{}", key.as_deref().unwrap_or(&String::default()));
        log::debug!("[FILES GET/POST/PUT/DELETE] processing for {}", &p);

        let user_password: &UsernamePassword = request.extensions().get().ok_or(Erro::RestAuthMissing)?;
        let method = request.method().clone();

        let (os, system) = {
            let mut ctrl = controller.lock().await;
            let system_manager = ctrl.system_manager_mut();
            let system = system_manager.system_credential(user_password.into()).await?.clone();

            (system.os()?.clone(), system)
        };

        if method == Method::GET && tokio::fs::metadata(&p).await?.is_dir() {
            log::debug!("[FILES GET] listing directories and files in {}", &p);
            let mut items = vec![];

            log::debug!("[FILES GET] collecting files and directories in {}", &p);
            for item in Dir::list(&p, &system).await? {
                let mut managed_by = vec![];

                if !item.directory() {
                    for managed_file_builder in controller.lock().await.file_builders() {
                        let path = std::path::Path::new(p.as_str());

                        log::trace!("[FILES GET] matching {:?}", path);

                        if managed_file_builder.r#match(
                            path.join(item.name())
                                .to_str()
                                .ok_or(Erro::PathInvalid)?,
                            &os,
                        ) {
                            let name = managed_file_builder.name().to_string();
                            log::trace!("[FILES GET] matched with {}", name);
                            managed_by.push(name);
                        }
                    }
                }

                log::trace!("[FILES GET] finished with item {}", item.name);

                items.push(DirItemExtended {
                    info: item,
                    managed_by,
                });
            }

            log::debug!("[FILES GET] sending list for {}", &p);
            return Ok(Json(items).into_response());
        };

        let mut ctrl = controller.lock().await;

        macro_rules! get_file {
            () => {
                if let Some(name) = query.name.as_deref() {
                    ctrl.file_builders_mut(name)?
                } else {
                    ctrl.file_builders_mut_by_match(&p, &system).await?
                }
            };
        }

        if method == Method::GET {
            let file = get_file!();
            log::debug!("[FILES GET] getting file {}", &p);
            Ok(Json(file.read(&p, &system).await?).into_response())
        } else if method == Method::DELETE {
            log::debug!("[FILES DELETE] deleting file {}", &p);
            let file = get_file!();
            file.delete(&p, &system).await?;
            Ok(StatusCode::ACCEPTED.into_response())
        } else if method == Method::POST {
            log::debug!("[FILES POST] write file {}", &p);
            let value: Json<Value> = request.extract().await?;
            let file = get_file!();
            file.write(&p, to_value(value.0)?, &system).await?;
            Ok(StatusCode::ACCEPTED.into_response())
        } else {
            log::error!("[FILES {}] invalid request method", &method);
            Err(Erro::HttpMethodNotAllowed(method))
        }
    }
}

/// Converts all errors into http status code and eventually a useful message
#[derive(Debug, Serialize)]
pub(crate) struct RestError {
    message: String,
}

impl IntoResponse for Erro {
    fn into_response(self) -> Response {
        let message = self.to_string();

        let code = match self {
            Erro::InvalidHeaderValue(_) |
            Erro::RestAuthMissing |
            Erro::AppBodyMissing |
            Erro::HttpMethodNotAllowed(_) |
            Erro::Base64Decode(_) |
            Erro::Deserialize(_)
            => StatusCode::BAD_REQUEST,

            Erro::TaskNotFound |
            Erro::AppNotFound |
            Erro::PathInvalid |
            Erro::FilesNotMatched |
            Erro::FilesNotMatchedByName(_) |
            Erro::FilesNotMatchedByPattern(_) |
            Erro::PathExistUnsupported
            => StatusCode::NOT_FOUND,

            Erro::OsDetectionFailed |
            Erro::AppIncompatible |
            Erro::TaskInvalidIndex |
            Erro::Io(_) |
            Erro::Regex(_) |
            Erro::FromUtf8(_) |
            Erro::DirFileSizeUnknown |
            Erro::File(_) |
            Erro::Hosts(_) |
            Erro::Mdstat(_) |
            Erro::Crypto(_) |
            Erro::LoadAvg(_) |
            Erro::Version(_) |
            Erro::Cron(_) |
            Erro::Uname(_) |
            Erro::Passwd(_) |
            Erro::Semver(_) |
            Erro::ParseInt(_) |
            Erro::SerdeJson(_) |
            Erro::Ssh(_) |
            Erro::ParseFloat(_) |
            Erro::JsonRejection(_) |
            Erro::ToStrError(_) |
            Erro::Http(_) |
            Erro::HyperError(_) |
            Erro::AsyncSsh(_) |
            Erro::Yaml(_) |
            Erro::AddrParse(_) |
            Erro::Join(_) |
            Erro::FileTypeUnknown(_) |
            Erro::FileTypeUnsupported |
            Erro::PrivateKeyPath |
            Erro::Rcgen(_) |
            Erro::Rustls(_) |
            Erro::Infallible(_) |
            Erro::SystemDetection |
            Erro::OsDetection |
            Erro::EndpointIncompatible |
            Erro::RunUserUnsupported(_) |
            Erro::ReadUserUnsupported(_) |
            Erro::ReadSshUnsupported(_) |
            Erro::WriteUserUnsupported(_) |
            Erro::WriteSshUnsupported(_) |
            Erro::DeleteUserUnsupported(_) |
            Erro::DeleteSshUnsupported(_) |
            Erro::RunUserStdin |
            Erro::RunUser(_, _) |
            Erro::RunSsh(_, _) |
            Erro::EndpointMissing |
            Erro::WriteUserTempPath |
            Erro::CertificatePath |
            Erro::OsRelease(_)
            => StatusCode::INTERNAL_SERVER_ERROR,

            Erro::AuthNotFound |
            Erro::AuthTokenExpired |
            Erro::RestAuthInvalid |
            Erro::RunUserUserInvalid |
            Erro::RunUserPasswordInvalid
            => StatusCode::UNAUTHORIZED,
        };

        log::error!("code {},  error {}", code, message);

        (code, Json(RestError {
            message
        })).into_response()
    }
}

// tokio mutex not serializable
#[derive(Serialize)]
struct TaskResult {
    id: usize,
    app_name: String,
    app_input: Value,
    app_output: Option<Value>,
    finished: bool,
}

#[derive(Debug, Serialize)]
struct DirItem {
    name: String,
    directory: bool,
    size: u64,
}

impl TryFrom<LsEntry> for DirItem {
    type Error = Erro;

    fn try_from(value: LsEntry) -> Resul<Self> {
        let directory = value.filename().ends_with('/');
        let name = if directory {
            value.filename()[..value.filename().len() - 1].to_string()
        } else {
            value.filename().to_string()
        };

        Ok(Self {
            name,
            directory,
            size: value.size().ok_or(Self::Error::DirFileSizeUnknown)?.parse()?,
        })
    }
}

impl DirItem {
    pub(crate) fn name(&self) -> &str { self.name.as_str() }
    pub(crate) fn directory(&self) -> bool { self.directory }
}

/// Manages directory listing
struct Dir;

impl Dir {
    pub(crate) async fn list<P: Into<PathBuf>>(path: P, exec: &System) -> Resul<Vec<DirItem>> {
        let p = path.into();
        let s = p.to_str().ok_or(Erro::PathInvalid)?;

        log::debug!("[LIST] getting directory list {}", s);
        LsApp::run_parse(LsInput::new(
            true, true, false, true, s,
        ), exec).await?
            .into_iter()
            .map(TryFrom::try_from)
            .collect::<Resul<Vec<DirItem>>>().map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::time::Duration;
    use axum::{middleware, Router};
    use axum::http::Request;
    use base64::Engine;
    use hyper::{Body, Method, StatusCode};
    use tokio::sync::Mutex;
    use crate::rest::{AppsBodyApp, auth, Rest, SharedController, TokenResult};
    use tower::ServiceExt;
    use crate::controller::Controller;
    use axum::body::HttpBody;
    use axum::response::Response;
    use serde::de::DeserializeOwned;
    use serde::Serialize;
    use serde_json::{json, Value};
    use tokio::fs::read_to_string;
    use crate::apps::AppBuilders;
    use crate::apps::sh::ShBuilder;
    use crate::utils::test::{PASSWORD, system_user, USERNAME};

    async fn get_body<T: DeserializeOwned>(result: Response) -> T {
        serde_json::from_slice(result.into_body().data().await.unwrap().unwrap().as_ref()).unwrap()
    }

    fn to_body<T: Serialize>(value: &T) -> Body {
        serde_json::to_vec(value).unwrap().into()
    }

    async fn request(app: Router, ctrl: SharedController, method: Method, body: Body, uri: &str) -> Response {
        let token_string = ctrl.lock()
            .await
            .auth_mut()
            .insert_or_replace(USERNAME.into(), PASSWORD.into());

        app.clone()
            .oneshot(Request::builder()
                .method(method)
                .uri(uri)
                .header("Authorization", "Bearer ".to_owned() + &token_string)
                .header("Content-Type", "application/json")
                .body(body)
                .unwrap())
            .await
            .unwrap()
    }

    async fn app() -> (Router, SharedController) {
        std::env::set_var("RUST_LOG", "trace");
        let _ = env_logger::builder().is_test(true).try_init();

        let ctrl = SharedController::new(Mutex::new(
            Controller::new(
                Duration::from_secs(100),
                None,
            ).await.unwrap()
        ));

        let router = Rest::routes()
            .with_state(ctrl.clone())
            .layer(middleware::from_fn_with_state(ctrl.clone(), auth));

        (router, ctrl)
    }

    #[tokio::test]
    async fn test_get_token() {
        let (app, ctrl) = app().await;

        let user_pass = base64
        ::engine
        ::general_purpose
        ::STANDARD.encode(format!("{}:{}", USERNAME, PASSWORD));
        let result = app
            .oneshot(Request::builder()
                .uri("/token")
                .header("Authorization", "Basic ".to_owned() + &user_pass)
                .body(Body::empty())
                .unwrap())
            .await
            .unwrap();
        assert!(ctrl.lock().await.auth_mut().get(&get_body::<TokenResult>(result).await.token).is_ok());
    }

    #[tokio::test]
    async fn test_auth_with_token_and_renew() {
        let (app, ctrl) = app().await;

        let token_string = ctrl.lock()
            .await
            .auth_mut()
            .insert_or_replace(USERNAME.into(), PASSWORD.into());

        let result = app
            .oneshot(Request::builder()
                .uri("/token")
                .header("Authorization", "Bearer ".to_owned() + &token_string)
                .body(Body::empty())
                .unwrap())
            .await
            .unwrap();

        let token: TokenResult = get_body(result).await;
        assert_ne!(token.token, token_string);
        assert!(ctrl.lock().await.auth_mut().get(&token.token).is_ok());
    }

    #[tokio::test]
    async fn test_get_token_failed() {
        let (app, _ctrl) = app().await;

        let user_pass = base64
        ::engine
        ::general_purpose
        ::STANDARD.encode(format!("{}:invalid", USERNAME));
        let result = app
            .oneshot(Request::builder()
                .uri("/token")
                .header("Authorization", "Basic ".to_owned() + &user_pass)
                .body(Body::empty())
                .unwrap())
            .await
            .unwrap();
        assert_eq!(result.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_auth_with_token_failed() {
        let (app, _ctrl) = app().await;
        let result = app
            .oneshot(Request::builder()
                .uri("/token")
                .header("Authorization", "Bearer invalid")
                .body(Body::empty())
                .unwrap())
            .await
            .unwrap();

        assert_eq!(result.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_delete_token() {
        let (app, ctrl) = app().await;

        let token_string = ctrl.lock()
            .await
            .auth_mut()
            .insert_or_replace(USERNAME.into(), PASSWORD.into());

        for code in [
            StatusCode::ACCEPTED,
            StatusCode::UNAUTHORIZED
        ] {
            let result = app.clone()
                .oneshot(Request::builder()
                    .method(Method::DELETE)
                    .uri("/token")
                    .header("Authorization", "Bearer ".to_owned() + &token_string)
                    .body(Body::empty())
                    .unwrap())
                .await
                .unwrap();

            assert_eq!(result.status(), code);
        }
    }

    #[tokio::test]
    async fn test_tasks() {
        let (app, ctrl) = app().await;

        let mut c = ctrl.lock().await;
        let tk = c.task_controller_mut();
        let mut task_result = tk.new_task(AppBuilders::ShBuilder(ShBuilder::default()),
                                          json!({
            "command": "sleep 3"
        }), system_user().await).await.unwrap();

        drop(c);

        task_result.as_object_mut().unwrap().insert("status".into(), Value::String("running".into())); // is already running in the meantime

        let result = request(app.clone(), ctrl.clone(), Method::GET, Body::empty(), "/tasks").await;
        let body: Value = get_body(result).await;
        assert_eq!(body, Value::Array(vec![task_result.clone()]));

        let result = request(app, ctrl, Method::GET, Body::empty(), "/tasks/1").await;
        let body: Value = get_body(result).await;
        assert_eq!(body, task_result);
    }

    #[tokio::test]
    async fn test_apps() {
        let (app, ctrl) = app().await;

        // help
        let result = request(app.clone(), ctrl.clone(), Method::GET, Body::empty(), "/apps").await;
        let body_result: Value = get_body(result).await;
        assert!(body_result.is_array());

        // multi
        let body = vec![
            AppsBodyApp {
                name: "ls".into(),
                input: json!({
                    "path": "/tmp"
                }),
            },
            AppsBodyApp {
                name: "ls".into(),
                input: json!({
                    "path": "/tmp"
                    }),
            },
        ];
        let result = request(app.clone(),
                             ctrl.clone(),
                             Method::POST,
                             to_body(&body),
                             "/apps").await;
        let body_result: Value = get_body(result).await;
        assert!(body_result.is_array());

        // multi async
        let result = request(app.clone(),
                             ctrl.clone(),
                             Method::POST,
                             to_body(&body),
                             "/apps?async=true").await;

        let body_result: Value = get_body(result).await;
        assert_eq!(body_result.as_array().unwrap().get(0).unwrap().as_object().unwrap().get("id").unwrap(), 1);
        assert_eq!(body_result.as_array().unwrap().get(1).unwrap().as_object().unwrap().get("id").unwrap(), 2);

        // single
        let body = json!({
                            "path": "/tmp"
                            });
        let result = request(app.clone(),
                             ctrl.clone(),
                             Method::POST,
                             to_body(&body),
                             "/apps/ls").await;
        let body_result: Value = get_body(result).await;
        assert!(body_result.is_array());

        // single async
        let result = request(app.clone(),
                             ctrl.clone(),
                             Method::POST,
                             to_body(&body),
                             "/apps/ls?async=true").await;
        let body_result: Value = get_body(result).await;
        assert_eq!((body_result).as_object().unwrap().get("id").unwrap(), 3);
    }

    #[tokio::test]
    async fn test_files() {
        let (app, ctrl) = app().await;

        // help
        let result = request(app.clone(), ctrl.clone(), Method::GET, Body::empty(), "/files").await;
        assert!(get_body::<Value>(result).await.is_array());

        // file list
        for path in [
            "/files",
            "/files/",
            "/files/tmp"
        ] {
            let result = request(app.clone(), ctrl.clone(), Method::GET, Body::empty(), path).await;
            assert!(get_body::<Value>(result).await.is_array());
        }

        let result = request(app.clone(),
                             ctrl.clone(),
                             Method::GET,
                             Body::empty(),
                             "/files/proc/uptime").await;
        assert!(get_body::<Value>(result).await.is_object());

        let path = "/tmp/createtestfile";
        let uri = "/files".to_owned() + path;

        let content = "text1";
        let result = request(app.clone(),
                             ctrl.clone(),
                             Method::POST,
                             to_body(&json!({
                                  "content": content
                              })),
                             &uri).await;
        assert_eq!(result.status(), StatusCode::ACCEPTED);
        assert_eq!(content, &read_to_string(path).await.unwrap());

        let content = "text2";
        let result = request(app.clone(),
                             ctrl.clone(),
                             Method::POST,
                             to_body(&json!({
                                  "content": content
                              })),
                             &uri).await;
        assert_eq!(result.status(), StatusCode::ACCEPTED);
        assert_eq!(content, &read_to_string(path).await.unwrap());

        let result = request(app.clone(),
                             ctrl.clone(),
                             Method::DELETE,
                             Body::empty(),
                             &uri).await;
        assert_eq!(result.status(), StatusCode::ACCEPTED);
        assert!(!Path::new(path).exists());

        // by name
        let result = request(app.clone(),
                             ctrl.clone(),
                             Method::GET,
                             Body::empty(),
                             "/files/etc/fstab?name=text").await;
        assert!(get_body::<Value>(result).await.is_string());

        // by invalid name
        let result = request(app.clone(),
                             ctrl.clone(),
                             Method::GET,
                             Body::empty(),
                             "/files/etc/fstab?name=invalid").await;
        assert_eq!(result.status(), StatusCode::NOT_FOUND);
    }
}