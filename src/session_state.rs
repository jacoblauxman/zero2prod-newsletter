use actix_session::{Session, SessionExt, SessionGetError, SessionInsertError};
use actix_web::dev::Payload;
use actix_web::{FromRequest, HttpRequest};
use std::future::{ready, Ready};
use uuid::Uuid;

// extension trait pattern ->
// allows strongly typed API 'on top' of `Session` - allows access to miodify state sans string keys and type casting in req handlers
pub struct TypedSession(Session);

impl TypedSession {
    const USER_ID_KEY: &'static str = "user_id";

    pub fn renew(&self) {
        self.0.renew();
    }

    pub fn insert_user_id(&self, user_id: Uuid) -> Result<(), SessionInsertError> {
        self.0.insert(Self::USER_ID_KEY, user_id)
    }

    pub fn get_user_id(&self) -> Result<Option<Uuid>, SessionGetError> {
        self.0.get(Self::USER_ID_KEY)
    }
}

// to enable req handlers to build instance of `TypedSession`
impl FromRequest for TypedSession {
    // convoluted way to say 'return same err returned by implementation of `FromRequest` for `Session`
    type Error = <Session as FromRequest>::Error;

    // Rust doesn't support `async` syntax in traits: `FromRequest` expects `Future` as return type
    // -> allows for etractors to perform async op's (HTTP call!)
    // No `Future` since no I/O, wrap in `Ready` to convert it to `Future` that resolves to wrapped value first time it's polled by executor
    type Future = Ready<Result<TypedSession, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        ready(Ok(TypedSession(req.get_session())))
    }
}
