use snafu::Snafu;

use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, new)]
#[serde(transparent)]
pub struct VideoId(holodex::model::id::VideoId);

impl VideoId {
    pub fn inner(&self) -> &holodex::model::id::VideoId {
        &self.0
    }
}

impl std::str::FromStr for VideoId {
    type Err = ParseVideoId;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        input
            .parse()
            .map(VideoId)
            .map_err(|_| ParseVideoId::new(input.to_string()))
    }
}

impl std::fmt::Display for VideoId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::convert::AsRef<str> for VideoId {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Snafu, new)]
#[snafu(display("Failed to parse video id: {}", text))]
pub struct ParseVideoId {
    pub text: String,
}
