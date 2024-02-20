use std::net::TcpListener;

#[tokio::test]
async fn health_check_works() {
    // Arrange
    let addr = spawn_app();
    let client = reqwest::Client::new();

    // Act
    let res = client
        .get(&format!("{}/health_check", &addr))
        .send()
        .await
        .expect("Failed to execute request");

    // Assert
    assert!(res.status().is_success());
    assert_eq!(Some(0), res.content_length());
}

// this helper creates app process and additionally returns our needed port-bound app address
fn spawn_app() -> String {
    // at OS level - trying to bind port 0 will have OS scan for available port to then bind our app instance!
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind to random port");
    let port = listener.local_addr().unwrap().port();
    let server = zero2prod::run(listener).expect("Failed to bind address");
    let _ = tokio::spawn(server);

    format!("http://127.0.0.1:{}", port)
}
