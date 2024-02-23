use unicode_segmentation::UnicodeSegmentation;

pub struct SubscriberName(String);

impl SubscriberName {
  // returns valid instance of `SubscriberName` if input given satisfies validation constraints - otherwise, panic
  pub fn parse(s: String) -> SubscriberName {
    let is_empty_or_whitespace = s.trim().is_empty();

    let is_too_long = s.graphemes(true).count() > 256;

    let forbidden_characters = ['/', '(', ')', '"', '<', '>', '\\', '{', '}'];
    let contains_forbidden_characters = s
        .chars()
        .any(|c| forbidden_characters.contains(&c));

      if is_empty_or_whitespace || is_too_long || contains_forbidden_characters {
        panic!("{} is not a valid subscriber name", s)
      } else {
        Self(s)
      }
  }
}

impl AsRef<str> for SubscriberName {
  fn as_ref(&self) -> &str {
    &self.0
  }
}


pub struct NewSubscriber {
  pub email: String,
  pub name: SubscriberName,
}
