use std::error::Error;
use std::fmt::{self, Debug, Display, Formatter};

use hyper::{Response, StatusCode};

use handler::IntoResponse;
use state::{request_id, State};
use http::response::create_response;

/// Describes an error which occurred during handler execution, and allows the creation of a HTTP
/// `Response`.
pub struct HandlerError {
    status_code: StatusCode,
    cause: Box<Error>,
}

/// Allows conversion into a HandlerError from an implementing type.
///
/// Futures returned from handlers can resolve to an error type with a value of `(State,
/// HandlerError)`.
///
/// ```rust
/// # extern crate gotham;
/// # extern crate futures;
/// #
/// # use std::fs::File;
/// # use gotham::state::State;
/// # use gotham::handler::{IntoHandlerError, HandlerFuture};
/// # use futures::future;
/// #
/// # #[allow(dead_code)]
/// fn my_handler(state: State) -> Box<HandlerFuture> {
///     match File::open("config.toml") {
///         Err(e) => Box::new(future::err((state, e.into_handler_error()))),
///         Ok(_) => // Create and return a response
/// #                unimplemented!(),
///     }
/// }
/// #
/// # fn main() {}
pub trait IntoHandlerError {
    /// Convert `self` into a `HandlerError`.
    ///
    /// The return value will have a `500 Internal Server Error` as the HTTP status code. See
    /// `HandlerError::with_status` for an example of changing it.
    fn into_handler_error(self) -> HandlerError;
}

impl<E> IntoHandlerError for E
where
    E: Error + 'static,
{
    fn into_handler_error(self) -> HandlerError {
        trace!(" converting Error to HandlerError: {}", self);

        HandlerError {
            status_code: StatusCode::InternalServerError,
            cause: Box::new(self),
        }
    }
}

impl Display for HandlerError {
    fn fmt(&self, out: &mut Formatter) -> fmt::Result {
        out.write_str("handler failed to process request")
    }
}

impl Debug for HandlerError {
    fn fmt(&self, out: &mut Formatter) -> fmt::Result {
        Display::fmt(self, out)?;
        out.write_str(" (")?;
        Debug::fmt(&*self.cause, out)?;
        out.write_str(")")
    }
}

impl Error for HandlerError {
    fn description(&self) -> &str {
        "handler failed to process request"
    }

    fn cause(&self) -> Option<&Error> {
        Some(&*self.cause)
    }
}

impl HandlerError {
    /// Sets the HTTP status code of the response which is generated by the `IntoResponse`
    /// implementation.
    ///
    /// ```rust
    /// # extern crate gotham;
    /// # extern crate hyper;
    /// # extern crate futures;
    /// #
    /// # use futures::future;
    /// # use hyper::StatusCode;
    /// # use gotham::state::State;
    /// # use gotham::handler::{IntoHandlerError, HandlerFuture};
    /// # use gotham::test::TestServer;
    /// #
    /// fn handler(state: State) -> Box<HandlerFuture> {
    ///     // It's OK if this is bogus, we just need something to convert into a `HandlerError`.
    ///     let io_error = std::io::Error::last_os_error();
    ///
    ///     let handler_error = io_error
    ///         .into_handler_error()
    ///         .with_status(StatusCode::ImATeapot);
    ///
    ///     Box::new(future::err((state, handler_error)))
    /// }
    ///
    /// # fn main() {
    /// #
    /// let test_server = TestServer::new(|| Ok(handler)).unwrap();
    /// let response = test_server.client().get("http://example.com/").perform().unwrap();
    /// assert_eq!(response.status(), StatusCode::ImATeapot);
    /// #
    /// # }
    /// ```
    pub fn with_status(self, status_code: StatusCode) -> HandlerError {
        HandlerError {
            status_code,
            ..self
        }
    }
}

impl IntoResponse for HandlerError {
    fn into_response(self, state: &State) -> Response {
        debug!(
            "[{}] HandlerError generating {} {} response: {}",
            request_id(state),
            self.status_code.as_u16(),
            self.status_code
                .canonical_reason()
                .unwrap_or("(unregistered)",),
            self.cause().map(|e| e.description()).unwrap_or("(none)"),
        );

        create_response(state, self.status_code, None)
    }
}
