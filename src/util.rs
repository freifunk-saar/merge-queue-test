use rocket::{
    Request,
    Outcome,
    State,
    request::{Outcome as ReqOutcome, FromRequest},
    response::Responder,
    http::Status,
};
use rocket_contrib::Template;

use failure::Error;
use lettre_email;

use config::Config;

/// Module for serde "with" to use hex encoding to byte arrays
pub mod hex_signing_key {
    use hex;
    use serde::{Deserializer, Deserialize, de::Error};
    use ring::{digest, hmac};

    pub fn deserialize<'de, D>(deserializer: D) -> Result<hmac::SigningKey, D::Error>
        where D: Deserializer<'de>
    {
         let bytes = hex::decode(String::deserialize(deserializer)?).map_err(|e| Error::custom(e))?;
         Ok(hmac::SigningKey::new(&digest::SHA256, bytes.as_slice()))
    }
}

/// Macro for generating URLs with query parameters
macro_rules! url_query {
    ( $url:expr, $( $name:ident = $value:expr ),* ) => {
        {
            let mut url = $url;
            url.query_pairs_mut()
                $(.append_pair(stringify!($name), $value.as_ref()))*;
            url
        }
    };
}

/// Horribly hacky hack to get access to the Request, and then a template's body, for building emails
pub struct EmailBuilder<'a, 'r: 'a> {
    request: &'a Request<'r>,
    config: &'a Config,
}

#[derive(Debug, Fail)]
enum ResponderError {
    #[fail(display = "responder failed with status {}", status)]
    RenderFailure {
        status: Status,
    },
    #[fail(display = "couldn't find a body")]
    NoBody,
}

impl<'a, 'r> FromRequest<'a, 'r> for EmailBuilder<'a, 'r> {
    type Error = ();
    fn from_request(request: &'a Request<'r>) -> ReqOutcome<Self, Self::Error> {
        let config = request.guard::<State<Config>>()?.inner();
        Outcome::Success(EmailBuilder { request, config })
    }
}

impl<'a, 'r> EmailBuilder<'a, 'r> {
    fn responder_body<'re, R: Responder<'re>>(&self, responder: R) -> Result<String, Error> {
        let mut resp = responder.respond_to(self.request)
            .map_err(|status| ResponderError::RenderFailure { status })?;
        Ok(resp.body_string().ok_or(ResponderError::NoBody)?)
    }

    /// Begin building an email from a template
    pub fn new(&self, email_template: Template) -> Result<lettre_email::EmailBuilder, Error> {
        let email_text = self.responder_body(email_template)?;
        let email_parts : Vec<&str> = email_text.splitn(3, '\n').collect();
        let (_, email_from, email_subject, email_body) = (email_parts[0], email_parts[1], email_parts[2], email_parts[3]);

        // Build email
        Ok(lettre_email::EmailBuilder::new()
            .from((self.config.ui.email_from.as_str(), email_from))
            .subject(email_subject)
            .text(email_body))
    }
}
