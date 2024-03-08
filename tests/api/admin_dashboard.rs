use crate::helpers::{assert_is_redirect_to, spawn_app};

#[tokio::test]
async fn must_be_logged_in_to_access_admin_dashboard() {
    // Arrange
    let app = spawn_app().await;

    // Act
    let res = app.get_admin_dashboard().await;

    // Assert
    assert_is_redirect_to(&res, "/login");
}
