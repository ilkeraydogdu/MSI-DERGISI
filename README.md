# MSI-DERGISI

## MSI Dergisi & MSI TDR Platform

Full-stack defense/aerospace publishing and e-commerce platform for Turkish defense industry.

**Domains:** savunmahaber.com / msidergisi.com

### Tech Stack
- **Backend:** Rust + Axum 0.7 + Tokio async runtime
- **Database:** SQLite (WAL mode), 22+ tables
- **Frontend:** Vanilla JS + jQuery + Bootstrap + GSAP/ScrollTrigger
- **Payment:** PayTR gateway
- **SEO:** IndexNow, sitemaps, RSS, JSON-LD Schema.org
- **Security:** WAF, CSP nonces, Argon2id hashing, Ammonia HTML sanitization

### Features
- Digital magazine kiosk with PDF DRM watermarking
- Flipbook reader
- B2B analytics engine
- Multi-language support (TR/EN/AR/DE/RU/FR)
- Dual-mode portal (Dergi/Haber)
- Admin CMS with visual design editor
- Subscription management

### Architecture
- 51 Rust source files
- ~60 REST API endpoints
- Server-side rendered pages with CSP nonce injection
- L1 in-memory cache (RwLock)

### Getting Started

\`\`\`bash
cargo build --release
./target/release/msi-dergisi-platform
\`\`\`

### Environment
Copy \`.env.example\` to \`.env\` and configure your settings.

### Testing

\`\`\`bash
cargo test --all
\`\`\`

---
*Built by UMSA Prodüksiyon*