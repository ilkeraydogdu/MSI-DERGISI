use super::*;

    #[test]
    fn detect_language_respects_query_cookie_accept_language_and_defaults_to_tr() {
        let mut params = HashMap::new();
        let mut headers = HeaderMap::new();
        assert_eq!(detect_language(&params, &headers), "tr");

        headers.insert(header::ACCEPT_LANGUAGE, "en-US,en;q=0.9,tr;q=0.8".parse().unwrap());
        assert_eq!(detect_language(&params, &headers), "en");

        headers.insert(header::ACCEPT_LANGUAGE, "tr-TR,tr;q=0.9,en-US;q=0.8".parse().unwrap());
        assert_eq!(detect_language(&params, &headers), "tr");

        headers.insert(header::ACCEPT_LANGUAGE, "en;q=0,tr;q=0.5".parse().unwrap());
        assert_eq!(detect_language(&params, &headers), "tr");

        headers.insert(header::ACCEPT_LANGUAGE, "en;q=0".parse().unwrap());
        assert_eq!(detect_language(&params, &headers), "tr");

        headers.insert(header::ACCEPT_LANGUAGE, "fr-FR,de;q=0.9".parse().unwrap());
        assert_eq!(detect_language(&params, &headers), "tr");

        headers.insert(header::ACCEPT_LANGUAGE, "en;q=invalid,tr;q=0.4".parse().unwrap());
        assert_eq!(detect_language(&params, &headers), "tr");

        headers.insert(header::ACCEPT_LANGUAGE, "en;q=0.8,tr;q=0.8".parse().unwrap());
        assert_eq!(detect_language(&params, &headers), "en");

        headers.insert(header::ACCEPT_LANGUAGE, "en-GB;q=0.9".parse().unwrap());
        headers.insert(header::COOKIE, "foo=1; msi_lang=tr".parse().unwrap());
        assert_eq!(detect_language(&params, &headers), "tr");

        headers.insert(header::COOKIE, "foo=1; msi_lang=en".parse().unwrap());
        assert_eq!(detect_language(&params, &headers), "en");

        params.insert("lang".to_string(), "tr".to_string());
        assert_eq!(detect_language(&params, &headers), "tr");
    }
