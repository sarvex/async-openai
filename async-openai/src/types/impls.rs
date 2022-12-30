use std::{
    fmt::Display,
    path::{Path, PathBuf},
};

use crate::{
    download::{download_url, save_b64},
    error::OpenAIError,
};

use super::{
    EmbeddingInput, ImageData, ImageInput, ImageResponse, ImageSize, Prompt, ResponseFormat, Stop,
};

macro_rules! impl_from {
    ($from_typ:ty, $to_typ:ty) => {
        impl From<$from_typ> for $to_typ {
            fn from(value: $from_typ) -> Self {
                <$to_typ>::String(value.into())
            }
        }

        impl From<Vec<$from_typ>> for $to_typ {
            fn from(value: Vec<$from_typ>) -> Self {
                <$to_typ>::StringArray(value.iter().map(|v| v.to_string()).collect())
            }
        }

        impl From<&Vec<$from_typ>> for $to_typ {
            fn from(value: &Vec<$from_typ>) -> Self {
                <$to_typ>::StringArray(value.iter().map(|v| v.to_string()).collect())
            }
        }

        impl<const N: usize> From<[$from_typ; N]> for $to_typ {
            fn from(value: [$from_typ; N]) -> Self {
                <$to_typ>::StringArray(value.into_iter().map(|v| v.to_string()).collect())
            }
        }

        impl<const N: usize> From<&[$from_typ; N]> for $to_typ {
            fn from(value: &[$from_typ; N]) -> Self {
                <$to_typ>::StringArray(value.into_iter().map(|v| v.to_string()).collect())
            }
        }
    };
}

// From String "family" to Prompt
impl_from!(&str, Prompt);
impl_from!(String, Prompt);
impl_from!(&String, Prompt);

// From String "family" to Stop
impl_from!(&str, Stop);
impl_from!(String, Stop);
impl_from!(&String, Stop);

// From String "family" to EmbeddingInput
impl_from!(&str, EmbeddingInput);
impl_from!(String, EmbeddingInput);
impl_from!(&String, EmbeddingInput);

impl Display for ImageSize {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                ImageSize::S256x256 => "256x256",
                ImageSize::S512x512 => "512x512",
                ImageSize::S1024x1024 => "1024x1024",
            }
        )
    }
}

impl Display for ResponseFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                ResponseFormat::Url => "url",
                ResponseFormat::B64Json => "b64_json",
            }
        )
    }
}

impl ImageInput {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        ImageInput {
            path: PathBuf::from(path.as_ref()),
        }
    }
}

impl<P: AsRef<Path>> From<P> for ImageInput {
    fn from(path: P) -> Self {
        Self {
            path: PathBuf::from(path.as_ref()),
        }
    }
}

impl ImageResponse {
    pub async fn save<P: AsRef<Path>>(&self, dir: P) -> Result<(), OpenAIError> {
        let exists = match Path::try_exists(dir.as_ref()) {
            Ok(exists) => exists,
            Err(e) => return Err(OpenAIError::FileSaveError(e.to_string())),
        };

        if !exists {
            std::fs::create_dir_all(dir.as_ref())
                .map_err(|e| OpenAIError::FileSaveError(e.to_string()))?;
        }

        let mut handles = vec![];
        for id in self.data.clone() {
            let dir_buf = PathBuf::from(dir.as_ref());
            handles.push(tokio::spawn(async move { id.save(dir_buf).await }));
        }

        let result = futures::future::join_all(handles).await;

        let errors: Vec<OpenAIError> = result
            .into_iter()
            .filter(|r| r.is_err() || r.as_ref().ok().unwrap().is_err())
            .map(|r| match r {
                Err(e) => OpenAIError::FileSaveError(e.to_string()),
                Ok(inner) => inner.err().unwrap(),
            })
            .collect();

        if errors.len() > 0 {
            Err(OpenAIError::FileSaveError(
                errors
                    .into_iter()
                    .map(|e| e.to_string())
                    .collect::<Vec<String>>()
                    .join("; "),
            ))
        } else {
            Ok(())
        }
    }
}

impl ImageData {
    async fn save<P: AsRef<Path>>(&self, dir: P) -> Result<(), OpenAIError> {
        match self {
            ImageData::Url(url) => download_url(url, dir).await?,
            ImageData::B64Json(b64_json) => save_b64(b64_json, dir).await?,
        }
        Ok(())
    }
}
