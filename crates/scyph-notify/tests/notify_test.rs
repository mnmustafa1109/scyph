use scyph_notify::{NoPushService, PushService, PushTemplate};
use std::sync::Arc;

struct TestPush {
    token: String,
    title: String,
    body: String,
}

impl PushTemplate for TestPush {
    fn token(&self) -> String {
        self.token.clone()
    }
    fn title(&self) -> String {
        self.title.clone()
    }
    fn body(&self) -> String {
        self.body.clone()
    }
}

#[tokio::test]
async fn test_push_template_send() {
    let push_service = Arc::new(NoPushService);
    let template = TestPush {
        token: "fcm_token_123".into(),
        title: "Hello".into(),
        body: "World".into(),
    };

    let res = push_service.send_template(&template).await;
    assert!(res.is_ok());
}

#[cfg(all(feature = "email", feature = "fcm"))]
mod composite_tests {
    use super::*;
    use scyph_notify::{
        CompositeNotification, EmailTemplate, NoEmailService, NotificationBroadcaster,
        TemplateEngine,
    };

    struct TestEmail {
        to: String,
        subject: String,
    }

    impl EmailTemplate for TestEmail {
        fn to(&self) -> Vec<String> {
            vec![self.to.clone()]
        }
        fn subject(&self) -> String {
            self.subject.clone()
        }
        fn template_name(&self) -> &str {
            "test.html"
        }
        fn context(&self) -> serde_json::Value {
            serde_json::json!({})
        }
    }

    struct TestEvent {
        token: String,
        email: String,
    }

    impl CompositeNotification for TestEvent {
        type Email = TestEmail;
        type Push = TestPush;

        fn email(&self) -> Option<Self::Email> {
            Some(TestEmail {
                to: self.email.clone(),
                subject: "Test Subject".into(),
            })
        }

        fn push(&self) -> Option<Self::Push> {
            Some(TestPush {
                token: self.token.clone(),
                title: "Test Title".into(),
                body: "Test Body".into(),
            })
        }
    }

    #[tokio::test]
    async fn test_broadcaster() {
        let email_service = Arc::new(NoEmailService);
        let push_service = Arc::new(NoPushService);

        let temp_dir = std::env::temp_dir().join("scyph_notify_test");
        let _ = std::fs::create_dir_all(&temp_dir);
        let test_file = temp_dir.join("test.html");
        let _ = std::fs::write(&test_file, "<h1>Hello</h1>");

        let glob = format!("{}/*.html", temp_dir.display());
        let engine = Arc::new(TemplateEngine::from_glob(&glob).unwrap());

        let broadcaster = Arc::new(NotificationBroadcaster::new(
            email_service,
            push_service,
            engine,
        ));

        let event = TestEvent {
            token: "token123".into(),
            email: "user@example.com".into(),
        };

        let res = broadcaster.broadcast(&event).await;
        assert!(res.is_ok());

        broadcaster.broadcast_background(event);

        let _ = std::fs::remove_file(test_file);
    }
}
