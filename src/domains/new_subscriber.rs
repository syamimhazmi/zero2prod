use crate::domains::subscriber_name::SubscriberName;
use crate::domains::subscriber_email::SubscriberEmail;

pub struct NewSubscriber {
    pub email: SubscriberEmail,
    pub name: SubscriberName,
}