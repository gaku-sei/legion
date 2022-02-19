macro_rules! assert_content_not_found {
    ($provider:expr, $id:expr) => {{
        match $provider.read_content(&$id).await {
            Ok(_) => panic!("content was found with the specified identifier `{}`", $id),
            Err(Error::NotFound {}) => {}
            Err(err) => panic!("unexpected error: {}", err),
        };
    }};
}

macro_rules! assert_read_content {
    ($provider:expr, $id:expr, $expected_content:expr) => {{
        let content = $provider
            .read_content(&$id)
            .await
            .expect("failed to read content");

        assert_eq!(content, $expected_content);
    }};
}

macro_rules! assert_write_content {
    ($provider:expr, $content:expr) => {{
        #[allow(clippy::string_lit_as_bytes)]
        $provider
            .write_content($content)
            .await
            .expect("failed to write content")
    }};
}

macro_rules! assert_write_avoided {
    ($provider:expr, $id:expr) => {{
        #[allow(clippy::string_lit_as_bytes)]
        match $provider.get_content_writer($id).await {
            Ok(_) => panic!(
                "content was written with the specified identifier `{}`",
                $id
            ),
            Err(Error::AlreadyExists {}) => {}
            Err(err) => panic!("unexpected error: {}", err),
        }
    }};
}

pub(crate) use {
    assert_content_not_found, assert_read_content, assert_write_avoided, assert_write_content,
};
