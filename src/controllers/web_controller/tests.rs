use super::*;

    #[test]
    fn detect_language_respects_query_cookie_and_defaults_to_tr() {
        let mut params = HashMap::new();
        let mut headers = HeaderMap::new();
        assert_eq!(detect_language(&params, &headers), "tr");

        headers.insert(header::ACCEPT_LANGUAGE, "en-US,en;q=0.9,tr;q=0.8".parse().unwrap());
        assert_eq!(detect_language(&params, &headers), "tr");

        headers.insert(header::ACCEPT_LANGUAGE, "tr-TR,tr;q=0.9,en-US;q=0.8".parse().unwrap());
        assert_eq!(detect_language(&params, &headers), "tr");

        headers.insert(header::COOKIE, "foo=1; msi_lang=en".parse().unwrap());
        assert_eq!(detect_language(&params, &headers), "en");

        params.insert("lang".to_string(), "tr".to_string());
        assert_eq!(detect_language(&params, &headers), "tr");
    }
