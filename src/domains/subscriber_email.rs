use validator::validate_email;

#[derive(Debug)]
pub struct SubscriberEmail(String);

impl SubscriberEmail {
    pub fn parse(value: String) -> Result<SubscriberEmail, String> {
        if validate_email(&value) {
            Ok(Self(value))
        } else {
            Err(format!("{} is not valid subscriber email", value))
        }
    }
}

impl AsRef<str> for SubscriberEmail {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod test {
    use super::SubscriberEmail;
    use claims::assert_err;

    #[test]
    fn empty_string_is_rejected() {
        let email = "".to_string();

        assert_err!(SubscriberEmail::parse(email));
    }

    #[test]
    fn email_missing_at_symbol_is_rejected() {
        let email = "usersubdomain.com".to_string();

        assert_err!(SubscriberEmail::parse(email));
    }

    #[test]
    fn email_missing_subject_is_rejected() {
        let email = "@domain.com".to_string();

        assert_err!(SubscriberEmail::parse(email));
    }
}