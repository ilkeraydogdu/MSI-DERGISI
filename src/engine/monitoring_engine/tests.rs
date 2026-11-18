use super::*;
use super::x509::cert_pattern_matches_domain;

    #[test]
    fn cpu_deltas_use_exact_jiffie_fields_without_fake_multipliers() {
        let prev = PrevCpuCounters {
            user: 1000,
            nice: 0,
            system: 500,
            idle: 8000,
            iowait: 300,
            irq: 0,
            softirq: 0,
            steal: 200,
        };
        let curr = PrevCpuCounters {
            user: 1100, // +100 (10%)
            nice: 0,
            system: 750, // +250 (25%)
            idle: 8500,  // +500 (50%)
            iowait: 400, // +100 (10%)
            irq: 0,
            softirq: 0,
            steal: 250, // +50  (5%)
        };
        let stats = compute_cpu_deltas(curr, Some(prev), [0.5, 0.4, 0.3]);
        assert_eq!(stats.user_pct, 10.0);
        assert_eq!(stats.system_pct, 25.0);
        assert_eq!(stats.idle_pct, 50.0);
        assert_eq!(stats.iowait_pct, 10.0);
        assert_eq!(stats.steal_pct, 5.0);
    }

    #[test]
    fn parses_nginx_combined_timestamp_accurately() {
        let dt = parse_nginx_timestamp("17/Nov/2026:19:30:14 +0300").expect("valid timestamp");
        assert_eq!(dt.format("%Y-%m-%d %H:%M:%S").to_string(), "2026-11-17 16:30:14");
    }

    #[test]
    fn wildcard_domain_matching_follows_rfc6125() {
        assert!(cert_pattern_matches_domain("*.msidergisi.com", "portal.msidergisi.com"));
        assert!(!cert_pattern_matches_domain("*.msidergisi.com", "msidergisi.com"));
        assert!(!cert_pattern_matches_domain("*.msidergisi.com", "a.b.msidergisi.com"));
        assert!(cert_pattern_matches_domain(
            "trusting-brahmagupta.194-110-169-165.plesk.page",
            "trusting-brahmagupta.194-110-169-165.plesk.page"
        ));
    }

    #[test]
    fn parses_minimal_der_x509_certificate_validity_and_san() {
        fn tlv(tag: u8, body: &[u8]) -> Vec<u8> {
            let mut out = vec![tag];
            if body.len() < 128 {
                out.push(body.len() as u8);
            } else {
                out.push(0x81);
                out.push(body.len() as u8);
            }
            out.extend_from_slice(body);
            out
        }

        // Build synthetic TBSCertificate DER with UTCTime validity & SAN dNSName
        let version = tlv(0xA0, &tlv(0x02, &[0x02]));
        let serial = tlv(0x02, &[0x01]);
        let alg = tlv(0x30, &tlv(0x06, &[0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D]));
        let cn_atv = tlv(
            0x30,
            &[tlv(0x06, &[0x55, 0x04, 0x03]), tlv(0x0C, b"msidergisi.com")].concat(),
        );
        let name = tlv(0x30, &tlv(0x31, &cn_atv));
        let validity = tlv(
            0x30,
            &[
                tlv(0x17, b"260101000000Z"),
                tlv(0x17, b"271231235959Z"),
            ]
            .concat(),
        );
        let spki = tlv(0x30, &[]);
        let san_gn = tlv(0x30, &tlv(0x82, b"*.plesk.page"));
        let san_ext = tlv(
            0x30,
            &[tlv(0x06, &[0x55, 0x1D, 0x11]), tlv(0x04, &san_gn)].concat(),
        );
        let exts = tlv(0xA3, &tlv(0x30, &san_ext));

        let tbs = tlv(
            0x30,
            &[version, serial, alg.clone(), name.clone(), validity, name, spki, exts].concat(),
        );
        let cert = tlv(0x30, &[tbs, alg, tlv(0x03, &[0x00])].concat());

        let (nb, na, names) = parse_x509_der_validity_and_names(&cert).expect("parsed X.509 DER");
        assert_eq!(nb.format("%Y-%m-%d").to_string(), "2026-01-01");
        assert_eq!(na.format("%Y-%m-%d").to_string(), "2027-12-31");
        assert!(names.contains(&"msidergisi.com".to_string()));
        assert!(names.contains(&"*.plesk.page".to_string()));
    }
