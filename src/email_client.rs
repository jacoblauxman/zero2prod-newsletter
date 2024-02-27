use crate::domain::SubscriberEmail;
use reqwest::Client;
use secrecy::{ExposeSecret, Secret};

pub struct EmailClient {
    http_client: Client,
    base_url: String,
    sender: SubscriberEmail,
    authorization_token: Secret<String>, // we don't want to log our api key on accident!
}

impl EmailClient {
    pub fn new(
        base_url: String,
        sender: SubscriberEmail,
        authorization_token: Secret<String>,
        timeout: std::time::Duration,
    ) -> Self {
        let http_client = Client::builder().timeout(timeout).build().unwrap();
        Self {
            // http_client: Client::new(),
            http_client,
            base_url,
            sender,
            authorization_token,
        }
    }

    pub async fn send_email(
        &self,
        recipient: SubscriberEmail,
        subject: &str,
        html_content: &str,
        text_content: &str,
    ) -> Result<(), reqwest::Error> {
        let url = &self.base_url;
        let req_body = SendEmailRequest {
            from: self.sender.as_ref(),
            to: recipient.as_ref(),
            subject,
            html_body: html_content,
            text_body: text_content,
        };

        // let builder =
        self.http_client
            .post(url)
            .header(
                "X-ElasticEmail-ApiKey",
                self.authorization_token.expose_secret(),
            )
            // able to use .json() method with json feature flag enabled with `reqwest` crate
            .json(&req_body)
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }
}

#[derive(serde::Serialize)]
// used due to field names requirement (ie `LikeThis`)
#[serde(rename_all = "PascalCase")]
struct SendEmailRequest<'a> {
    // to store ref's (of str slices) we add lifetime param 'a
    from: &'a str,
    to: &'a str,
    subject: &'a str,
    html_body: &'a str,
    text_body: &'a str,
}

#[cfg(test)]
mod tests {

    use crate::domain::SubscriberEmail;
    use crate::email_client::EmailClient;
    use fake::faker::internet::en::SafeEmail;
    use fake::faker::lorem::en::{Paragraph, Sentence};
    use fake::{Fake, Faker};
    use secrecy::Secret;
    // use tracing_subscriber::FmtSubscriber;
    use wiremock::matchers::any;
    #[allow(unused_imports)] // since not using 'path' explicitly right now
    use wiremock::matchers::{header, header_exists, method, path};
    use wiremock::Request;
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use claims::{assert_err, assert_ok};

    // custom matcher for confirming request body's data shape / fields correct
    struct SendEmailBodyMatcher;

    impl wiremock::Match for SendEmailBodyMatcher {
        fn matches(&self, req: &Request) -> bool {
            // parse body as json value
            let res: Result<serde_json::Value, _> = serde_json::from_slice(&req.body);

            if let Ok(body) = res {
                // dbg!(&body);
                // check all mandatory fields populated
                body.get("From").is_some()
                    && body.get("To").is_some()
                    && body.get("Subject").is_some()
                    && body.get("HtmlBody").is_some()
                    && body.get("TextBody").is_some()
            } else {
                // parsing failed - don't match req
                false
            }
        }
    }

    // helpers for gen'ing mock request data

    fn subject() -> String {
        Sentence(1..2).fake()
    }

    fn content() -> String {
        Paragraph(1..10).fake()
    }

    fn email() -> SubscriberEmail {
        SubscriberEmail::parse(SafeEmail().fake()).unwrap()
    }

    fn email_client(base_url: String) -> EmailClient {
        EmailClient::new(
            base_url,
            email(),
            Secret::new(Faker.fake()),
            std::time::Duration::from_millis(200),
        )
    }

    #[tokio::test]
    async fn send_email_sends_expected_req() {
        // Arrange
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri());

        Mock::given(header_exists("X-ElasticEmail-ApiKey"))
            .and(header("Content-Type", "application/json"))
            // .and(path("/email")) // not used with Elastic Email, example of Postmark's from zero2prod
            .and(method("POST"))
            // using custom body matcher for req checking via wiremock::Match
            .and(SendEmailBodyMatcher)
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;

        // Act
        let _ = email_client
            .send_email(email(), &subject(), &content(), &content())
            .await;

        // Assert
        // NOTE: Mock expectations are checked on `drop`
    }

    #[tokio::test]
    async fn send_email_succeeds_if_server_returns_200() {
        // Arrange
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri());

        // bare minimum needed to trigger path we want to test in `send_email`
        Mock::given(any())
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;

        // Act
        let res = email_client
            .send_email(email(), &subject(), &content(), &content())
            .await;

        // Assert
        assert_ok!(res);
    }

    #[tokio::test]
    async fn send_email_fails_if_server_returns_500() {
        // Arrange
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri());

        Mock::given(any())
            .respond_with(ResponseTemplate::new(500))
            .expect(1)
            .mount(&mock_server)
            .await;

        // Act
        let res = email_client
            .send_email(email(), &subject(), &content(), &content())
            .await;

        // Assert
        assert_err!(res);
    }

    #[tokio::test]
    async fn send_email_times_out_if_server_takes_too_long() {
        // Arrange
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri());

        let res = ResponseTemplate::new(200)
            // 3 min time delay:
            .set_delay(std::time::Duration::from_secs(180));
        Mock::given(any())
            .respond_with(res)
            .expect(1)
            .mount(&mock_server)
            .await;

        // Act
        let res = email_client
            .send_email(email(), &subject(), &content(), &content())
            .await;

        // Assert
        assert_err!(res);
    }
}
