use super::*;
use chumsky::{error::SimpleReason, prelude::*, text::whitespace};
use itertools::Itertools;

#[derive(Debug, Snafu)]
#[snafu(display("cannot parse '{input}' - {}", self.combine_errors("\n")))]
pub struct ParseError {
    input: String,
    errors: Vec<Simple<char>>,
}

impl ParseError {
    fn combine_errors(&self, separator: &str) -> String {
        self.errors
            .iter()
            .map(|err| {
                format!(
                    "{}:\n   {}",
                    err,
                    match err.reason() {
                        SimpleReason::Custom(msg) => format!("error {}", msg),
                        SimpleReason::Unexpected => "unexpected input".to_string(),
                        SimpleReason::Unclosed { span, delimiter } => {
                            format!(
                                "unclosed delimiter ({}..{}) in {}",
                                span.start, span.end, delimiter
                            )
                        }
                    }
                )
            })
            .join(separator)
    }
}

pub fn parse(input: &str) -> Result<Action, ParseError> {
    let parser = program().parse(input).map_err(|errors| ParseError {
        input: input.to_string(),
        errors,
    })?;

    Ok(parser)
}

fn program() -> impl Parser<char, Action, Error = Simple<char>> {
    action_add()
        .or(action_update())
        .or(action_remove())
        .or(action_list())
        .or(action_exit())
        .or(action_restart())
        .then_ignore(end())
}

fn action_add() -> impl Parser<char, Action, Error = Simple<char>> {
    just("add")
        .then_ignore(whitespace().at_least(1))
        .ignore_then(tracker_content())
        .map(|option| Action::Add { option })
}

fn action_update() -> impl Parser<char, Action, Error = Simple<char>> {
    just("update")
        .then_ignore(whitespace().at_least(1))
        .ignore_then(tracker_descriptor())
        .then_ignore(whitespace().at_least(1))
        .then(tracker_content())
        .map(|(tracker_id, option)| Action::Update { tracker_id, option })
}

fn action_remove() -> impl Parser<char, Action, Error = Simple<char>> {
    just("remove")
        .then_ignore(whitespace().at_least(1))
        .ignore_then(tracker_descriptor())
        .map(|tracker_id| Action::Remove { tracker_id })
}

fn action_list() -> impl Parser<char, Action, Error = Simple<char>> {
    just("list").to(Action::List)
}

fn action_exit() -> impl Parser<char, Action, Error = Simple<char>> {
    choice((just("exit"), just("quit"))).to(Action::Exit)
}

fn action_restart() -> impl Parser<char, Action, Error = Simple<char>> {
    just("restart").to(Action::Restart)
}

fn tracker_descriptor() -> impl Parser<char, TrackerId, Error = Simple<char>> {
    filter(char::is_ascii)
        .repeated()
        .at_least(1)
        .try_map(|str, span| {
            str.into_iter()
                .collect::<String>()
                .parse()
                .map_err(|_| Simple::custom(span, "invalid tracker id"))
        })
}

fn tracker_content() -> impl Parser<char, UpdateTracker, Error = Simple<char>> {
    take_until(end()).try_map(|(chars, _), span| {
        let str = chars.into_iter().collect::<String>();
        serde_json::from_str(&str)
            .map_err(|source| Simple::custom(span, format!("invalid tracker content: {}", source)))
    })
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use uuid::uuid;

    use crate::model::{tracker_id, TrackDuration};

    use super::*;

    #[test]
    fn test_grammar() {
        let action = program()
            .parse("remove fb71bb1a-f489-5b4b-9bc0-851e1901f2f4")
            .unwrap();

        assert_eq!(
            action,
            Action::Remove {
                tracker_id: tracker_id(uuid!("fb71bb1a-f489-5b4b-9bc0-851e1901f2f4"))
            }
        );
    }

    #[test]
    fn parse_tracker_id() {
        let result = tracker_descriptor()
            .parse("fc396eb2-1b9b-5a33-8a6d-4b1f47d82551")
            .unwrap();
        assert_eq!(result, tracker_id(uuid!("fc396eb2-1b9b-5a33-8a6d-4b1f47d82551")));
    }

    #[test]
    fn parse_tracker_content() {
        let result = tracker_content()
            .parse(
                json!({
                    "video_id": "fDiJSE0CrZ0",
                    "track_duration": 10
                })
                .to_string(),
            )
            .unwrap();
        assert_eq!(result.video_id, "fDiJSE0CrZ0".parse().unwrap());
        assert_eq!(result.track_target, None);
        assert_eq!(result.track_duration, TrackDuration::from_seconds(10));
    }
}
