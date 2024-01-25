use unicode_segmentation::UnicodeSegmentation;

pub struct NewSubscriber {
    pub email: String,
    pub name: SubscriberName,
}

pub struct SubscriberName(String);

impl SubscriberName {
    pub fn parse(value: String) -> SubscriberName {
        // `.trim()` returns a view over the input `s` without trailing
        // whitespace-like characters.
        // `.is_empty` checks if the view contains any character.
        let is_empty_or_whitespace = value.trim().is_empty();

        // A grapheme is defined by the Unicode standard as a "user-perceived"
        // character: `å` is a single grapheme, but it is composed of two characters
        // (`a` and `̊`).
        //
        // `graphemes` returns an iterator over the graphemes in the input `s`.
        // `true` specifies that we want to use the extended grapheme definition set,
        // the recommended one.
        let is_too_long = value.graphemes(true).count() > 256;

        let forbidden_characters = ['/', '(', ')', '"', '<', '>', '\\', '{', '}'];
        let contains_forbidden_characters = value.chars()
            .any(|g| forbidden_characters.contains(&g));

        if ! is_empty_or_whitespace || is_too_long || contains_forbidden_characters {
            Self(value)
        } else {
            panic!("{} is not a valid subscriber name", value)
        }
    }
}