#!/usr/bin/env python3.11
"""
Synchronize live data from https://msidergisi.com/ into MSI Dergisi SQLite DB,
download missing cover images into public/images/msi/, align bilingual
design_config settings, and regenerate storage/seed/live_msi_archive.json.
"""

from __future__ import annotations

import concurrent.futures
import html as html_lib
import json
import mimetypes
import os
import re
import sqlite3
import sys
import time
import unicodedata
import urllib.error
import urllib.parse
import urllib.request
import xml.etree.ElementTree as ET
from datetime import datetime, timezone
from pathlib import Path

BASE_URL = "https://msidergisi.com"
ROOT_DIR = Path(__file__).resolve().parent.parent
DB_PATH = ROOT_DIR / "storage" / "db" / "msi_platform.db"
PUBLIC_DIR = ROOT_DIR / "public"
IMG_DIR = PUBLIC_DIR / "images" / "msi"
ARCHIVE_JSON_PATH = ROOT_DIR / "storage" / "seed" / "live_msi_archive.json"

HEADERS = {
    "User-Agent": (
        "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 "
        "(KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36"
    ),
    "Accept": "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
    "Accept-Language": "tr-TR,tr;q=0.9,en-US;q=0.8,en;q=0.7",
}

CAPTCHA_MARKERS = (
    "cf-turnstile",
    "g-recaptcha",
    "hcaptcha",
    "verify you are human",
    "/cdn-cgi/challenge-platform",
)

MILESTONES_TR = [
    ("2004", "MSI Dergisi'nin ilk sayısının yayınlanması"),
    ("2006", "İlk İngilizce Sayı: DSA Malezya Fuarı"),
    ("2013", "MSI Dergisi'nin 100'üncü sayısının yayınlanması"),
    ("2014", "MSI TDR'nin ilk sayısının yayınlanması"),
    ("2015", "AMAC Magazine'in ilk sayısının yayınlanması"),
    ("2020", "MSI Dergisi'nin 200'üncü sayısının yayınlanması"),
    ("2021", "MSI TDR'nin 100'üncü sayısının yayınlanması"),
    ("2022", "MSI Dergisi'nin 230'uncu sayısının yayınlanması"),
    ("2023", "MSI Dergisi'nin 245'inci sayısının yayınlanması"),
    ("2024", "MSI Dergisi'nin 250'inci sayısının yayınlanması"),
]

MILESTONES_EN = [
    ("2004", "Publication of the first issue of MSI Magazine (MSI Dergisi)"),
    ("2006", "First English Special Issue: DSA Malaysia Defence Exhibition"),
    ("2013", "Publication of the 100th issue of MSI Dergisi"),
    ("2014", "Launch of the inaugural issue of MSI Turkish Defence Review (MSI TDR)"),
    ("2015", "Launch of the inaugural issue of AMAC Magazine"),
    ("2020", "Publication of the 200th issue of MSI Dergisi"),
    ("2021", "Publication of the 100th issue of MSI TDR"),
    ("2022", "Publication of the 230th issue of MSI Dergisi"),
    ("2023", "Publication of the 245th issue of MSI Dergisi"),
    ("2024", "Publication of the 250th issue of MSI Dergisi"),
]


def http_get_bytes(url: str, timeout: int = 25, retries: int = 2) -> bytes:
    last_err: Exception | None = None
    for attempt in range(retries + 1):
        try:
            req = urllib.request.Request(url, headers=HEADERS)
            with urllib.request.urlopen(req, timeout=timeout) as resp:
                data = resp.read()
                ctype = (resp.headers.get("Content-Type") or "").lower()
                if "text/" in ctype or "json" in ctype or "xml" in ctype:
                    low = data[:8192].decode("utf-8", "ignore").lower()
                    for marker in CAPTCHA_MARKERS:
                        if marker in low:
                            raise RuntimeError(f"CAPTCHA/anti-bot gate encountered at {url}: {marker}")
                return data
        except RuntimeError:
            raise
        except Exception as exc:
            last_err = exc
            if attempt < retries:
                time.sleep(0.6 * (attempt + 1))
    raise RuntimeError(f"Failed GET {url}: {last_err}")


def fetch_inertia_props(url: str) -> dict:
    raw = http_get_bytes(url).decode("utf-8", "replace")
    m = re.search(r'<script[^>]*data-page="app"[^>]*>(.*?)</script>', raw, re.S)
    if m:
        return json.loads(m.group(1)).get("props", {})
    m2 = re.search(r'data-page="(\{.*?\})"', raw, re.S)
    if m2:
        return json.loads(html_lib.unescape(m2.group(1))).get("props", {})
    raise ValueError(f"No Inertia payload at {url}")


def norm_title(text: str) -> str:
    if not text:
        return ""
    s = unicodedata.normalize("NFKC", text).lower()
    tr_map = str.maketrans("çğıöşüâîû", "cgiosuaiu")
    s = s.translate(tr_map)
    s = re.sub(r"[^\w\s]", " ", s)
    s = re.sub(r"\s+", " ", s).strip()
    return s


def parse_published_at(raw_dt: str | None, fallback_dmy: str | None) -> str:
    if raw_dt:
        s = raw_dt.strip()
        m = re.match(r"(\d{4}-\d{2}-\d{2})[T ](\d{2}:\d{2}:\d{2})", s)
        if m:
            return f"{m.group(1)} {m.group(2)}"
        m2 = re.match(r"(\d{4}-\d{2}-\d{2})", s)
        if m2:
            return f"{m2.group(1)} 12:00:00"
    if fallback_dmy:
        m = re.match(r"(\d{2})\.(\d{2})\.(\d{4})", fallback_dmy.strip())
        if m:
            return f"{m.group(3)}-{m.group(2)}-{m.group(1)} 12:00:00"
    return datetime.now(timezone.utc).strftime("%Y-%m-%d %H:%M:%S")


def estimate_read_time(html_body: str) -> int:
    text = re.sub(r"<[^>]+>", " ", html_body or "")
    words = len(text.split())
    return max(1, round(words / 180))


def pick_image_url(img_obj: dict | None) -> str | None:
    if not isinstance(img_obj, dict):
        return None
    url = img_obj.get("url")
    if isinstance(url, str) and url.startswith("http"):
        return url
    file_rel = img_obj.get("file")
    uuid_str = img_obj.get("uuid")
    if isinstance(file_rel, str) and isinstance(uuid_str, str):
        return f"{BASE_URL}/media/{uuid_str}/{file_rel}"
    return None


def download_image_for_slug(slug: str, url: str) -> str | None:
    IMG_DIR.mkdir(parents=True, exist_ok=True)
    safe_slug = re.sub(r"[^a-zA-Z0-9_-]", "-", slug)[:110].strip("-") or "msi-asset"
    parsed_path = urllib.parse.urlparse(url).path
    ext = os.path.splitext(parsed_path)[1].lower()
    if ext not in (".webp", ".jpg", ".jpeg", ".png", ".gif"):
        ext = ".webp"
    rel_path = f"images/msi/{safe_slug}{ext}"
    dest = PUBLIC_DIR / rel_path
    if dest.exists() and dest.stat().st_size > 200:
        return rel_path
    try:
        data = http_get_bytes(url, timeout=25, retries=1)
        if len(data) < 200 or data[:9].lower().startswith(b"<!doctype"):
            return None
        dest.write_bytes(data)
        return rel_path
    except Exception:
        return None


def crawl_category_all_pages(base_cat_url: str, cat_slug: str) -> tuple[dict, list[dict]]:
    first = fetch_inertia_props(f"{base_cat_url}?page=1")
    posts_obj = first.get("posts") or {}
    meta = posts_obj.get("meta") or {}
    last_page = int(meta.get("last_page") or 1)
    cat_info = first.get("category") or {}
    items = list(posts_obj.get("data") or [])
    if last_page > 1:
        with concurrent.futures.ThreadPoolExecutor(max_workers=6) as pool:
            futs = [
                pool.submit(fetch_inertia_props, f"{base_cat_url}?page={p}")
                for p in range(2, last_page + 1)
            ]
            for fut in concurrent.futures.as_completed(futs):
                try:
                    d = fut.result()
                    items.extend((d.get("posts") or {}).get("data") or [])
                except Exception as exc:
                    print(f"  WARN page error on {cat_slug}: {exc}")
    return cat_info, items


def classify_content_type_and_source(cat_slug: str, placements: set[str], existing_ct: str | None = None) -> tuple[str, str]:
    c_slug = (cat_slug or "").strip().lower()
    ordered = [
        "kapak-dosyasi",
        "basyazilar",
        "soylesiler",
        "ozel-haberler",
        "tanitici-makaleler",
        "makaleler-analizler",
        "mesajlar",
        "advertorials",
        "yeni-sayi-kapak",
        "haberler",
    ]
    chosen_ct = c_slug if c_slug in ordered else None
    if not chosen_ct:
        for cand in ordered:
            if cand in placements:
                chosen_ct = cand
                break
    if not chosen_ct and existing_ct:
        ct_map = {
            "advertorial": "advertorials",
            "basyazi": "basyazilar",
            "kapak-dosyalari": "kapak-dosyasi",
        }
        chosen_ct = ct_map.get(existing_ct, existing_ct)
    if not chosen_ct:
        chosen_ct = "haberler"

    if chosen_ct in ("kapak-dosyasi", "basyazilar", "soylesiler", "ozel-haberler", "mesajlar", "tanitici-makaleler", "makaleler-analizler", "advertorials") or "ozel-dosya" in placements or "dergi-icerikleri" in placements:
        sg = "dergi-icerigi"
    elif chosen_ct == "yeni-sayi-kapak":
        sg = "sadece-online"
    else:
        sg = "gunluk-haber"
    return chosen_ct, sg


def translate_nav_and_design_en(tr_cfg: dict, existing_en_cfg: dict) -> dict:
    cfg = json.loads(json.dumps(tr_cfg))
    cfg["navigation"] = [
        {"label": "News", "href": "/kategori/haberler?lang=en"},
        {"label": "Cover Files", "href": "/kategori/kapak-dosyasi?lang=en"},
        {"label": "Interviews", "href": "/kategori/soylesiler?lang=en"},
        {"label": "Special Reports", "href": "/kategori/ozel-haberler?lang=en"},
        {"label": "Editorial Comments", "href": "/kategori/basyazilar?lang=en"},
        {"label": "Messages", "href": "/kategori/mesajlar?lang=en"},
        {"label": "Special Coverages", "href": "/kategori/tanitici-makaleler?lang=en"},
        {"label": "Advertorials", "href": "/kategori/advertorials?lang=en"},
        {"label": "Special File", "href": "/kategori/ozel-dosya?lang=en"},
    ]

    sec_link_tr_to_en = {
        "Tümünü gör": "View all",
        "Tüm haberler": "All news",
        "Tüm haber akışı": "All news feed",
        "Haber arşivine git": "Browse news archive",
        "Dergi moduna geç": "Switch to Magazine mode",
        "Abonelik yetkilerini incele": "Explore subscription plans",
        "Kurumsal teklif alın": "Request corporate quote",
        "Dijital kütüphaneyi keşfet": "Explore digital library",
        "Tüm kapak dosyaları": "All cover files",
        "Tüm söyleşiler": "All interviews",
        "Tüm özel haberler": "All special reports",
        "Tüm başyazılar": "All editorials",
        "Tüm tanıtıcı makaleler": "All special coverages",
        "Tüm mesajlar": "All messages",
        "Tüm makale ve analizler": "All articles & analyses",
        "Tüm kurumsal tanıtımlar": "All advertorials",
        "Tüm arşivi incele": "Explore full archive",
    }

    sec_tr_to_en = {
        "Savunmanın aklı.\nGeleceğin doktrini.": (
            "Defence Intelligence.\nStrategic Horizon.",
            "MSI TURKISH DEFENCE REVIEW · GLOBAL DEFENCE REFERENT SINCE 2004",
            "The international English edition carrying Turkish defence, aerospace, naval, and security engineering breakthroughs to global procurement authorities and attachés.",
            "Explore Digital Library",
        ),
        "Kritik gelişmeler ve saha gündemi": (
            "Critical Developments & Field Dispatches",
            "DEFENCE NEWS · LIVE FEED",
            "Real-time breaking news, official procurement announcements, and international export milestones.",
            None,
        ),
        "Programlar, sözleşmeler ve teknolojik eşikler": (
            "Programmes, Contracts & Technological Milestones",
            "LATEST INTELLIGENCE",
            None,
            "All news feed",
        ),
        "Derinlikli saha ve etkinlik dosyaları": (
            "In-Depth Field & Exhibition Dossiers",
            "SPECIAL REPORTS",
            None,
            "All special reports",
        ),
        "Gündemi teknoloji dikeyine göre filtreleyin": (
            "Filter News by Defence Technology Domain",
            "STRATEGIC DOMAINS",
            None,
            None,
        ),
        "Stratejik radarda öne çıkanlar": (
            "Featured Highlights on the Strategic Radar",
            "NEWS ARCHIVE",
            None,
            "Browse news archive",
        ),
        "Manşetin ardındaki doktrini okuyun.": (
            "Read the Doctrine Behind the Headlines.",
            "MAGAZINE EDITION",
            "Explore deep engineering analyses and exclusive interviews in the MSI Turkish Defence Review monthly edition.",
            "Switch to Magazine mode",
        ),
        "Sayının kalbinden stratejik kapak dosyaları": (
            "Strategic Cover Stories & Technical Dossiers",
            "COVER FILES & DEEP ANALYSES",
            None,
            None,
        ),
        "Karar vericiler ve sektör liderleriyle birinci elden vizyon": (
            "First-hand Vision from Decision Makers & Industry Leaders",
            "EXCLUSIVE INTERVIEWS",
            None,
            "All interviews",
        ),
        "Saha analizleri, uluslararası fuarlar ve özel savunma dosyaları": (
            "Field Dispatches, Global Expos & Special Defence Dossiers",
            "SPECIAL DOSSIERS & EXHIBITIONS",
            None,
            "All dossiers",
        ),
        "Editoryal perspektif ve stratejik muhasebe": (
            "Editorial Perspective & Strategic Outlook",
            "EDITORIAL COMMENTS",
            None,
            None,
        ),
        "Mühendislik kabiliyetleri, alt sistemler ve küresel çözümler": (
            "Engineering Capabilities, Subsystems & Global Solutions",
            "SPECIAL COVERAGES",
            None,
            None,
        ),
        "Bakanlık, tedarik makamı ve komuta kademesi mesajları": (
            "Messages from Ministries, Procurement Authorities & Commands",
            "OFFICIAL MESSAGES",
            None,
            None,
        ),
        "Doktrin, harp tarihi ve geleceğin kuvvet yapısı": (
            "Doctrine, Warfare History & Future Force Structure",
            "ARTICLES & ANALYSES",
            None,
            None,
        ),
        "Savunma sanayii kurumsal vitrini ve teknoloji lansmanları": (
            "Defence Industry Corporate Showcase & Launches",
            "ADVERTORIALS",
            None,
            None,
        ),
        "2004'ten bugüne savunma ve havacılık hafızası": (
            "Defence & Aerospace Archive Since 2004",
            "DIGITAL MAGAZINE KIOSK",
            "Browse 20+ years of MSI Dergisi and MSI TDR issues in our digital archive.",
            "Open E-Magazine Kiosk",
        ),
        "Uzmanlık alanınıza göre istihbarat akışı": (
            "Intelligence Feed by Defence Sector",
            "14 DEFENCE CATALOG DOMAINS",
            None,
            None,
        ),
        "Stratejik bilgiye tam yetkiyle hükmedin.": (
            "Command Strategic Defence Intelligence Without Limits.",
            "INSTITUTIONAL & INDIVIDUAL ACCESS",
            "Access print & digital MSI TDR issues, technical dossiers, and verified briefings.",
            "View Subscription Plans",
        ),
        "Güncel ve geçmiş referans sayılar": (
            "Current & Archive Reference Issues",
            "DIGITAL KIOSK & ARCHIVE",
            "Browse every historical issue of MSI Dergisi and MSI Turkish Defence Review.",
            None,
        ),
        "Abonelik ve erişim planları": (
            "Subscription & Access Plans",
            "MSI READER CLUB",
            None,
            None,
        ),
        "Referans kitaplar, almanaklar ve özel yayınlar": (
            "Reference Books, Almanacs & Special Publications",
            "INSTITUTIONAL LIBRARY SELECTION",
            None,
            None,
        ),
        "Çok kullanıcılı kurumsal lisans mı arıyorsunuz?": (
            "Looking for a multi-seat institutional license?",
            "ENTERPRISE ACCESS",
            "Contact our defence media desk for diplomatic, military, and enterprise bundles.",
            "Request corporate quote",
        ),
    }

    corp_en = {
        "kurumsal-kunye": {
            "title": "Official Masthead & Publisher Statement",
            "description": "Official masthead of MSI Dergisi (ISSN 1306-0309) and MSI Turkish Defence Review (MSI TDR) published by UMSA Production Ltd.",
            "sections": [
                {
                    "kind": "text",
                    "eyebrow": "MANAGEMENT & EDITORIAL BOARD",
                    "title": "Owner, Editorial Leadership & Contact",
                    "description": (
                        "Publications: MSI Dergisi & MSI Turkish Defence Review (MSI TDR) / savunmahaber.com\n"
                        "ISSN: 1306-0309\n\n"
                        "Owner & Editor-in-Chief: Ümit BAYRAKTAR — ubayraktar@savunmahaber.com\n"
                        "News Editor & General Coordinator: Naile BAYRAKTAR — n.bayraktar@savunmahaber.com\n"
                        "Managing Editor: Dr. K. Burak CODUR — b.codur@savunmahaber.com\n\n"
                        "Editorial Desk:\n"
                        "• haber.ekip@savunmahaber.com • news@savunmahaber.com\n"
                        "• Emre ÜRE — e.ure@savunmahaber.com\n"
                        "• Sena ŞAHAN — s.sahan@savunmahaber.com\n"
                        "• Ahmet Sezai KARASAZOĞLU — a.karasazoglu@savunmahaber.com\n\n"
                        "Business Development & Subscriptions:\n"
                        "• Project & Business Development Coordinator: Özay ÖYMEZ — o.oymez@savunmahaber.com\n"
                        "• Corporate Relations & Subscriptions: Refika TURGUT — r.turgut@savunmahaber.com\n\n"
                        "Editorial Advisory Board:\n"
                        "Prof. Dr. Süleyman TOLUN, Prof. Dr. Adil YÜKSELEN, Prof. Dr. Okan ADDEMİR, Prof. Dr. Zahit MECİTOĞLU, "
                        "Prof. Dr. Mikdat KADIOĞLU, Prof. Dr. Gonca COŞKUN, Prof. Dr. Yıldırım SALDIRANER, Assoc. Prof. Burak ÇINAR, Prof. Dr. Ş. Hakan ATAPEK"
                    ),
                },
                {
                    "kind": "text",
                    "eyebrow": "HEADQUARTERS & PUBLISHER ADDRESS",
                    "title": "UMSA Production Publishing Ltd.",
                    "description": (
                        "Address: Yaşamkent Mah. 3147 Sok. No:10, 06810 Çankaya / Ankara — TÜRKİYE\n"
                        "Phone: +90 312 225 41 73 • Fax: +90 312 225 41 74\n"
                        "Email: info@savunmahaber.com • Web: https://www.msidergisi.com"
                    ),
                },
            ],
        },
        "kurumsal-hakkimizda": {
            "title": "About Us — Who We Are?",
            "description": "The reference Turkish and English monthly publications covering defence, aerospace and security technologies since 2004.",
            "sections": [
                {
                    "kind": "text",
                    "eyebrow": "ESTABLISHED IN 2004 · 20+ YEARS · 400+ ISSUES",
                    "title": "The Global Reference Voice of the Turkish Defence & Aerospace Sector",
                    "description": (
                        "Our journey began in 2004 with the goal of bringing reference publications to readers that effectively "
                        "promote the Turkish defence and aerospace industry domestically and abroad.\n\n"
                        "After more than 20 rewarding years and over 400 issues published for our readers, we continue on our path with confidence:\n\n"
                        "• 2004 — MSI Dergisi launched (ISSN 1306-0309)\n"
                        "• 2014 — MSI Turkish Defence Review (MSI TDR) launched in English for international readers\n"
                        "• 2015 — AMAC Magazine joined our family as the official publication for foreign military attachés in Türkiye\n"
                        "• 2019 — KasabaWorks Advertising & Corporate Communications Agency established\n\n"
                        "Built on first-hand primary-source reporting, our print magazines, digital E-Magazine library, and web portal serve "
                        "decision-makers, procurement authorities, armed forces personnel, academia, and industry partners."
                    ),
                }
            ],
        },
        "kurumsal-yolculugumuz": {
            "title": "Our Journey (2004 – 2024 Milestones)",
            "description": "Two decades of strategic defence journalism milestones from 2004 to today.",
            "sections": [
                {
                    "kind": "text",
                    "eyebrow": "HISTORICAL MILESTONES (2004 – 2024)",
                    "title": "Milestones",
                    "description": (
                        "As a pioneering reference publication in the Turkish defence ecosystem, we preserve institutional memory across key milestones:\n\n"
                        + "\n".join(f"• {yr} — {lbl}" for yr, lbl in MILESTONES_EN)
                        + "\n\nMSI Dergisi delivers monthly Turkish analysis connecting end users, procurement authorities, universities, clusters, and prime contractors.\n\nMSI TDR (Turkish Defence Review) has been bringing developments in the Turkish defence and aerospace industry in English to international stakeholders since 2014."
                    ),
                }
            ],
        },
        "kurumsal-kvkk": {
            "title": "Personal Data Protection Law (KVKK) Disclosure Notice",
            "description": "Official Information Notice by MSI Dergisi under Law No. 6698 on the Protection of Personal Data (KVKK).",
            "sections": [
                {
                    "kind": "text",
                    "eyebrow": "LAW NO. 6698 ARTICLE 10 DISCLOSURE",
                    "title": "Data Controller & Processing Purposes",
                    "description": (
                        "Pursuant to Article 10 of Law No. 6698 on the Protection of Personal Data ('KVKK'), this Disclosure Notice "
                        "is issued by MSI Dergisi (UMSA Prodüksiyon Basım ve Yayıncılık Ltd. Şti.) as Data Controller.\n\n"
                        "• Data Controller: MSI Dergisi\n"
                        "• Address: Yaşamkent Mah. 3147 Sok. No:10, Çankaya / Ankara — TÜRKİYE\n"
                        "• Phone: +90 312 225 41 73\n"
                        "• Email: info@savunmahaber.com\n"
                        "• Website: https://www.msidergisi.com\n\n"
                        "Personal data is processed solely for delivering publication and E-Magazine services, managing institutional "
                        "verification and subscriptions, fulfilling tax and legal obligations (Laws 213, 5651, 6502), ensuring cybersecurity, "
                        "and—where explicitly consented—sending editorial newsletters. Under KVKK Article 11, you may submit requests regarding your "
                        "personal data in writing or via info@savunmahaber.com and receive a response within 30 days."
                    ),
                }
            ],
        },
    }

    for page in cfg.get("pages", []):
        slug = page.get("slug", "")
        if slug == "magazine":
            page["title"] = "MSI Turkish Defence Review (MSI TDR) — Defence & Aerospace Portal"
            page["description"] = "English international edition of MSI Dergisi covering Turkish defence, aerospace, naval systems, and global procurement news."
        elif slug == "news":
            page["title"] = "savunmahaber.com / MSI TDR News — Real-Time Defence Feed"
            page["description"] = "Daily breaking news, official statements, and international exports in Turkish defence and aerospace."
        elif slug == "e-dergi":
            page["title"] = "E-Magazine Archive — MSI Dergisi & MSI TDR Issues"
        elif slug == "abonelik-ve-satis":
            page["title"] = "Subscriptions & Defence Publications Store"

        if slug in corp_en:
            page["title"] = corp_en[slug]["title"]
            page["description"] = corp_en[slug]["description"]
            page["sections"] = corp_en[slug]["sections"]
        else:
            if slug == "magazine":
                page["sections"] = [
                    sec for sec in page.get("sections", [])
                    if sec.get("category") != "makaleler-analizler"
                ]
            for sec in page.get("sections", []):
                tr_t = (sec.get("title") or "").strip()
                tr_ll = (sec.get("link_label") or "").strip()
                if tr_t in sec_tr_to_en:
                    t_en, e_en, d_en, ll_en = sec_tr_to_en[tr_t]
                    sec["title"] = t_en
                    if e_en:
                        sec["eyebrow"] = e_en
                    if d_en:
                        sec["description"] = d_en
                    if ll_en:
                        sec["link_label"] = ll_en
                    elif tr_ll in sec_link_tr_to_en:
                        sec["link_label"] = sec_link_tr_to_en[tr_ll]
                elif tr_ll in sec_link_tr_to_en:
                    sec["link_label"] = sec_link_tr_to_en[tr_ll]
                if sec.get("link") and sec["link"].startswith("/") and "lang=" not in sec["link"]:
                    sep = "&" if "?" in sec["link"] else "?"
                    sec["link"] = f"{sec['link']}{sep}lang=en"

    return cfg


def align_tr_corporate_pages(tr_cfg: dict) -> dict:
    cfg = json.loads(json.dumps(tr_cfg))
    milestones_block = "\n".join(f"• {yr} — {lbl}" for yr, lbl in MILESTONES_TR)
    for page in cfg.get("pages", []):
        slug = page.get("slug")
        if slug == "kurumsal-yolculugumuz":
            page["description"] = "Türk savunma sanayisinin öncü referans yayını olarak 2004'ten bugüne uzanan kilometre taşlarımız."
            page["sections"] = [
                {
                    "kind": "text",
                    "eyebrow": "KİLOMETRE TAŞLARI (2004 – 2024)",
                    "title": "Tarihsel Gelişim",
                    "description": (
                        "Türk savunma sanayisinin öncü kuruluşu olarak, yılların deneyimi ve teknolojik yenilikleriyle geleceği şekillendiriyoruz.\n\n"
                        + milestones_block
                        + "\n\nMSI Dergisi: Sektöre ilişkin güncel gelişmeleri her ay ve Türkçe olarak okurlarına sunan MSI Dergisi; "
                        "kullanıcı, tedarik makamı, üniversiteler ve araştırma merkezleri, kümelenmeler, ana yükleniciler ve alt yükleniciler "
                        "arasında stratejik bilgi köprüsü kurmaktadır.\n\n"
                        "MSI TDR (Turkish Defence Review): 2014 yılından bu yana sektörün yurt dışındaki paydaşlarına ve uluslararası okuyucularına "
                        "İngilizce olarak ulaşan referans dergimizdir."
                    ),
                }
            ]
        elif slug == "kurumsal-kvkk" and page.get("sections"):
            d0 = page["sections"][0].get("description", "")
            if "MSI Dergisi" not in d0:
                page["sections"][0]["description"] = d0.replace(
                    "UMSA Prodüksiyon Basım ve Yayıncılık Ltd. Şti.",
                    "MSI Dergisi (UMSA Prodüksiyon Basım ve Yayıncılık Ltd. Şti.)",
                )
    return cfg


def main() -> int:
    print(f"[1/6] Fetching sitemap & homepages from {BASE_URL} ...")
    sm_bytes = http_get_bytes(f"{BASE_URL}/sitemap.xml")
    sm_root = ET.fromstring(sm_bytes)
    ns = {"sm": "http://www.sitemaps.org/schemas/sitemap/0.9"}
    all_sm_urls = [loc.text.strip() for loc in sm_root.findall(".//sm:loc", ns) if loc.text]

    tr_sm_slugs: set[str] = set()
    en_sm_slugs: set[str] = set()
    cat_urls_tr: set[str] = set()
    cat_urls_en: set[str] = set()

    for u in all_sm_urls:
        if "/en/haberler/" in u:
            en_sm_slugs.add(u.rstrip("/").split("/")[-1])
        elif "/haberler/" in u:
            tr_sm_slugs.add(u.rstrip("/").split("/")[-1])
        elif "/en/kategoriler/" in u:
            cat_urls_en.add(u.rstrip("/"))
        elif "/kategoriler/" in u:
            cat_urls_tr.add(u.rstrip("/"))

    home_tr = fetch_inertia_props(f"{BASE_URL}/")
    home_en = fetch_inertia_props(f"{BASE_URL}/en")

    # Gather category metadata and homepage section placements
    cat_meta: dict[str, dict] = {}
    for c in home_tr.get("categories", []) + home_tr.get("webNavigationMenus", []):
        slug = (c.get("link") or "").rstrip("/").split("/")[-1]
        if slug:
            cat_meta.setdefault(slug, {})["name_tr"] = c.get("name") or slug
            cat_meta[slug]["desc_tr"] = c.get("description") or ""
            cat_urls_tr.add(f"{BASE_URL}/kategoriler/{slug}")
    for c in home_en.get("categories", []) + home_en.get("webNavigationMenus", []):
        slug = (c.get("link") or "").rstrip("/").split("/")[-1]
        if slug:
            cat_meta.setdefault(slug, {})["name_en"] = c.get("name") or slug
            cat_urls_en.add(f"{BASE_URL}/en/kategoriler/{slug}")

    tr_placements: dict[str, set[str]] = {}
    en_placements: dict[str, set[str]] = {}

    for sec in home_tr.get("content", []):
        key = (sec.get("key") or "").strip()
        if key:
            cat_meta.setdefault(key, {}).setdefault("name_tr", sec.get("title") or key)
            cat_urls_tr.add(f"{BASE_URL}/kategoriler/{key}")
        for it in sec.get("items", []):
            sl = it.get("slug")
            if sl:
                tr_sm_slugs.add(sl)
                if key:
                    tr_placements.setdefault(sl, set()).add(key)

    for sec in home_en.get("content", []):
        key = (sec.get("key") or "").strip()
        if key:
            cat_meta.setdefault(key, {}).setdefault("name_en", sec.get("title") or key)
            cat_urls_en.add(f"{BASE_URL}/en/kategoriler/{key}")
        for it in sec.get("items", []):
            sl = it.get("slug")
            if sl:
                en_sm_slugs.add(sl)
                if key:
                    en_placements.setdefault(sl, set()).add(key)

    print(
        f"  Discovered TR sitemap slugs={len(tr_sm_slugs)}, EN sitemap slugs={len(en_sm_slugs)}, "
        f"TR cats={len(cat_urls_tr)}, EN cats={len(cat_urls_en)}"
    )

    print("[2/6] Crawling all paginated TR & EN category archives ...")
    for cu in sorted(cat_urls_tr):
        cat_slug = cu.rstrip("/").split("/")[-1]
        try:
            cinfo, posts = crawl_category_all_pages(cu, cat_slug)
            if cinfo.get("name"):
                cat_meta.setdefault(cat_slug, {}).setdefault("name_tr", cinfo["name"])
            for p in posts:
                sl = p.get("slug")
                if sl:
                    tr_sm_slugs.add(sl)
                    tr_placements.setdefault(sl, set()).add(cat_slug)
            print(f"  TR /{cat_slug}: {len(posts)} items")
        except Exception as exc:
            print(f"  WARN TR cat {cat_slug}: {exc}")

    for cu in sorted(cat_urls_en):
        cat_slug = cu.rstrip("/").split("/")[-1]
        try:
            cinfo, posts = crawl_category_all_pages(cu, cat_slug)
            if cinfo.get("name"):
                cat_meta.setdefault(cat_slug, {}).setdefault("name_en", cinfo["name"])
            for p in posts:
                sl = p.get("slug")
                if sl:
                    en_sm_slugs.add(sl)
                    en_placements.setdefault(sl, set()).add(cat_slug)
            print(f"  EN /{cat_slug}: {len(posts)} items")
        except Exception as exc:
            print(f"  WARN EN cat {cat_slug}: {exc}")

    conn = sqlite3.connect(DB_PATH)
    conn.row_factory = sqlite3.Row
    cur = conn.cursor()

    # Upsert categories cleanly
    print("[3/6] Upserting live categories ...")
    existing_cat_slugs = {r["slug"] for r in cur.execute("SELECT slug FROM categories")}
    next_sort = 50
    for cslug, info in sorted(cat_meta.items()):
        ntr = info.get("name_tr") or cslug
        nen = info.get("name_en") or ntr
        dtr = info.get("desc_tr") or ""
        gtype = "etkinlik-dosyasi" if any(k in cslug for k in ("idef", "igef", "saha", "sedec", "amac", "nato", "milmast", "ni")) else "icerikler"
        if cslug not in existing_cat_slugs:
            cur.execute(
                """INSERT OR IGNORE INTO categories
                   (slug, parent_slug, name_tr, name_en, group_type, description_tr, description_en, icon_class, sort_order, is_dynamic)
                   VALUES (?, NULL, ?, ?, ?, ?, ?, 'fa-newspaper-o', ?, 1)""",
                (cslug, ntr, nen, gtype, dtr, nen, next_sort),
            )
            next_sort += 1
        elif cslug in ("highlights", "feature", "ozel-dosya", "advertorials", "basyazilar", "kapak-dosyasi", "yeni-sayi-kapak"):
            cur.execute(
                "UPDATE categories SET name_tr = COALESCE(NULLIF(name_tr, ''), ?), name_en = ? WHERE slug = ?",
                (ntr, nen, cslug),
            )
    conn.commit()

    # Load existing DB contents
    db_by_slug: dict[str, dict] = {}
    db_by_norm_title: dict[str, dict] = {}
    for row in cur.execute("SELECT * FROM contents"):
        d = dict(row)
        db_by_slug[d["slug"]] = d
        nt = norm_title(d["title_tr"])
        if nt:
            db_by_norm_title.setdefault(nt, d)

    # Fetch missing/incomplete TR posts concurrently
    tr_to_fetch = [
        sl
        for sl in sorted(tr_sm_slugs)
        if sl not in db_by_slug or len((db_by_slug[sl].get("body_tr") or "").strip()) < 80
    ]
    print(f"[4/6] Fetching {len(tr_to_fetch)} missing/short TR articles & {len(en_sm_slugs)} EN checks ...")

    tr_posts_data: dict[str, dict] = {}
    if tr_to_fetch:
        with concurrent.futures.ThreadPoolExecutor(max_workers=8) as pool:
            futs = {
                pool.submit(fetch_inertia_props, f"{BASE_URL}/haberler/{urllib.parse.quote(sl)}"): sl
                for sl in tr_to_fetch
            }
            for fut in concurrent.futures.as_completed(futs):
                sl = futs[fut]
                try:
                    props = fut.result()
                    if props.get("post"):
                        tr_posts_data[sl] = props["post"]
                except Exception as exc:
                    print(f"  WARN TR post {sl}: {exc}")

    # Insert / Reconcile TR posts (including hashed-slug matches)
    created_tr = 0
    reconciled_hash = 0
    for sl, post in tr_posts_data.items():
        title = (post.get("title") or "").strip()
        summary = (post.get("subTitle") or "").strip()
        body = (post.get("content") or "").strip()
        cat_slug = ((post.get("category") or {}).get("slug") or "haberler").strip()
        if cat_slug:
            tr_placements.setdefault(sl, set()).add(cat_slug)
        pls = tr_placements.get(sl, set())
        ct, sg = classify_content_type_and_source(cat_slug, pls)
        is_sponsored = 1 if ("advertorial" in ct or "advertorials" in pls) else 0
        pub_at = parse_published_at(post.get("publishedAt"), post.get("date"))
        author_obj = post.get("author") or {}
        author_name = f"{author_obj.get('name', '')} {author_obj.get('last_name', '')}".strip() or "MSI Editöryal Masası"
        img_url = pick_image_url(post.get("img"))
        cover = download_image_for_slug(sl, img_url) if img_url else None
        if not cover:
            cover = "images/msi/dergi-286.webp"
        read_min = estimate_read_time(body)

        nt = norm_title(title)
        if sl in db_by_slug:
            cur.execute(
                """UPDATE contents
                   SET title_tr = ?, summary_tr = ?, body_tr = ?, content_type = ?, source_group = ?,
                       cover_image = CASE WHEN cover_image = '' THEN ? ELSE cover_image END,
                       published_at = ?, is_sponsored = MAX(is_sponsored, ?)
                   WHERE slug = ?""",
                (title, summary, body, ct, sg, cover, pub_at, is_sponsored, sl),
            )
        elif nt in db_by_norm_title and sl not in db_by_slug:
            old_row = db_by_norm_title[nt]
            old_slug = old_row["slug"]
            cur.execute(
                """UPDATE contents
                   SET slug = ?, title_tr = ?, summary_tr = COALESCE(NULLIF(?, ''), summary_tr),
                       body_tr = CASE WHEN length(?) > length(body_tr) THEN ? ELSE body_tr END,
                       published_at = ?, is_sponsored = MAX(is_sponsored, ?)
                   WHERE id = ?""",
                (sl, title, summary, body, body, pub_at, is_sponsored, old_row["id"]),
            )
            cur.execute(
                "INSERT OR IGNORE INTO seo_redirects (old_path, new_path, status_code, hit_count) VALUES (?, ?, 301, 0)",
                (f"/icerik/{old_slug}", f"/icerik/{sl}"),
            )
            cur.execute(
                "INSERT OR IGNORE INTO seo_redirects (old_path, new_path, status_code, hit_count) VALUES (?, ?, 301, 0)",
                (f"/haberler/{old_slug}", f"/icerik/{sl}"),
            )
            db_by_slug.pop(old_slug, None)
            old_row["slug"] = sl
            db_by_slug[sl] = old_row
            reconciled_hash += 1
        else:
            cur.execute(
                """INSERT INTO contents (
                       slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
                       content_type, source_group, cover_image, author_name, author_title,
                       issue_number, published_at, read_time_min, visibility_level, single_price_try,
                       is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
                       view_count, true_read_count, status
                   ) VALUES (?, ?, '', ?, '', ?, '', ?, ?, ?, ?, 'Savunma Sanayii Analisti',
                             NULL, ?, ?, 0, 0.0, ?, NULL, ?, 0, NULL, 1, 1, 'published')""",
                (
                    sl,
                    title,
                    summary,
                    body,
                    ct,
                    sg,
                    cover,
                    author_name,
                    pub_at,
                    read_min,
                    is_sponsored,
                    "ADVERTORIAL" if is_sponsored else None,
                ),
            )
            new_id = cur.lastrowid
            db_by_slug[sl] = {"id": new_id, "slug": sl, "title_tr": title, "title_en": "", "body_tr": body, "body_en": ""}
            created_tr += 1
    conn.commit()
    print(f"  TR inserted={created_tr}, hashed-slug reconciled={reconciled_hash}")

    # Now fetch EN posts needing creation or EN column reconciliation
    en_to_fetch = []
    for sl in sorted(en_sm_slugs):
        row = db_by_slug.get(sl)
        if not row:
            en_to_fetch.append(sl)
        elif (
            (row.get("source_group") or "") != "msi-tdr"
            and sl not in tr_sm_slugs
        ) or len((row.get("body_en") or "").strip()) < 80:
            en_to_fetch.append(sl)

    en_posts_data: dict[str, dict] = {}
    if en_to_fetch:
        with concurrent.futures.ThreadPoolExecutor(max_workers=8) as pool:
            futs = {
                pool.submit(fetch_inertia_props, f"{BASE_URL}/en/haberler/{urllib.parse.quote(sl)}"): sl
                for sl in en_to_fetch
            }
            for fut in concurrent.futures.as_completed(futs):
                sl = futs[fut]
                try:
                    props = fut.result()
                    if props.get("post"):
                        en_posts_data[sl] = props["post"]
                except Exception as exc:
                    print(f"  WARN EN post {sl}: {exc}")

    created_en = 0
    updated_en = 0
    for sl, post in en_posts_data.items():
        en_title = (post.get("title") or "").strip()
        en_summary = (post.get("subTitle") or "").strip()
        en_body = (post.get("content") or "").strip()
        cat_slug = ((post.get("category") or {}).get("slug") or "haberler").strip()
        if cat_slug:
            en_placements.setdefault(sl, set()).add(cat_slug)
        pls = en_placements.get(sl, set())
        ct, _ = classify_content_type_and_source(cat_slug, pls)
        is_sponsored = 1 if ("advertorial" in ct or "advertorials" in pls) else 0
        pub_at = parse_published_at(post.get("publishedAt"), post.get("date"))
        author_obj = post.get("author") or {}
        author_name = f"{author_obj.get('name', '')} {author_obj.get('last_name', '')}".strip() or "MSI TDR Editorial Desk"
        img_url = pick_image_url(post.get("img"))
        read_min = estimate_read_time(en_body)

        if sl in db_by_slug:
            is_bilingual_shared = sl in tr_sm_slugs
            if is_bilingual_shared:
                cur.execute(
                    "UPDATE contents SET title_en = ?, summary_en = ?, body_en = ? WHERE slug = ?",
                    (en_title, en_summary, en_body, sl),
                )
            else:
                cur.execute(
                    """UPDATE contents
                       SET title_tr = COALESCE(NULLIF(title_tr, ''), ?),
                           title_en = ?,
                           summary_tr = COALESCE(NULLIF(summary_tr, ''), ?),
                           summary_en = ?,
                           body_tr = COALESCE(NULLIF(body_tr, ''), ?),
                           body_en = ?,
                           source_group = 'msi-tdr',
                           content_type = ?,
                           published_at = COALESCE(NULLIF(published_at, ''), ?)
                       WHERE slug = ?""",
                    (en_title, en_title, en_summary, en_summary, en_body, en_body, ct, pub_at, sl),
                )
            updated_en += 1
        else:
            cover = (download_image_for_slug(sl, img_url) if img_url else None) or "images/msi/dergi-286.webp"
            cur.execute(
                """INSERT INTO contents (
                       slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
                       content_type, source_group, cover_image, author_name, author_title,
                       issue_number, published_at, read_time_min, visibility_level, single_price_try,
                       is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
                       view_count, true_read_count, status
                   ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, 'msi-tdr', ?, ?, 'Defence Industry Analyst',
                             NULL, ?, ?, 0, 0.0, ?, NULL, ?, 0, NULL, 1, 1, 'published')""",
                (
                    sl,
                    en_title,
                    en_title,
                    en_summary,
                    en_summary,
                    en_body,
                    en_body,
                    ct,
                    cover,
                    author_name,
                    pub_at,
                    read_min,
                    is_sponsored,
                    "ADVERTORIAL" if is_sponsored else None,
                ),
            )
            created_en += 1

    # Also catch any remaining existing pure-EN rows in en_sm_slugs not in tr_sm_slugs
    for sl in en_sm_slugs:
        if sl not in tr_sm_slugs:
            cur.execute("UPDATE contents SET source_group = 'msi-tdr' WHERE slug = ? AND source_group != 'msi-tdr'", (sl,))

    conn.commit()
    print(f"  EN created={created_en}, updated={updated_en}")

    # Sync placements from all TR & EN category/homepage crawls
    slug_to_id = {r["slug"]: r["id"] for r in cur.execute("SELECT id, slug FROM contents")}
    pl_count = 0
    for sl, pset in list(tr_placements.items()) + list(en_placements.items()):
        cid = slug_to_id.get(sl)
        if not cid:
            continue
        for pslug in pset:
            if not pslug:
                continue
            cur.execute(
                "INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?, ?)",
                (cid, pslug),
            )
            pl_count += cur.rowcount

    # Normalize advertorials flag & legacy content_type aliases into placements
    cur.execute("UPDATE contents SET is_sponsored = 1 WHERE content_type IN ('advertorial', 'advertorials')")
    conn.commit()
    print(f"  Added {pl_count} new content_placements")

    # Audit cover_image files on disk
    print("[5/6] Auditing all cover images on disk under public/ ...")
    broken_rows = []
    for r in cur.execute("SELECT id, slug, cover_image FROM contents"):
        ci = (r["cover_image"] or "").lstrip("/")
        fp = PUBLIC_DIR / ci if ci else None
        if not ci or not fp or not fp.exists() or fp.stat().st_size < 100:
            broken_rows.append((r["id"], r["slug"], ci))

    repaired_imgs = 0
    if broken_rows:
        def repair_one(item):
            cid, sl, _ = item
            is_en = sl in en_sm_slugs and sl not in tr_sm_slugs
            page_url = f"{BASE_URL}{'/en' if is_en else ''}/haberler/{urllib.parse.quote(sl)}"
            try:
                props = fetch_inertia_props(page_url)
                u = pick_image_url((props.get("post") or {}).get("img"))
                if u:
                    rel = download_image_for_slug(sl, u)
                    if rel:
                        return (cid, rel)
            except Exception:
                pass
            return (cid, "images/msi/dergi-286.webp")

        with concurrent.futures.ThreadPoolExecutor(max_workers=6) as pool:
            for cid, new_rel in pool.map(repair_one, broken_rows):
                cur.execute("UPDATE contents SET cover_image = ? WHERE id = ?", (new_rel, cid))
                repaired_imgs += 1

    # Also ensure every magazine_issue cover_image exists
    for r in cur.execute("SELECT issue_number, cover_image FROM magazine_issues"):
        ci = (r["cover_image"] or "").lstrip("/")
        fp = PUBLIC_DIR / ci if ci else None
        if not ci or not fp or not fp.exists() or fp.stat().st_size < 100:
            cur.execute("UPDATE magazine_issues SET cover_image = 'images/msi/dergi-286.webp' WHERE issue_number = ?", (r["issue_number"],))

    # Ensure design_config has aligned TR corporate info + full English translation
    row_dc = cur.execute("SELECT value_tr, value_en FROM site_settings WHERE key = 'design_config'").fetchone()
    if row_dc:
        tr_cfg = align_tr_corporate_pages(json.loads(row_dc["value_tr"]))
        en_cfg = translate_nav_and_design_en(tr_cfg, json.loads(row_dc["value_en"]))
        now_iso = datetime.now(timezone.utc).isoformat()
        cur.execute(
            "UPDATE site_settings SET value_tr = ?, value_en = ?, updated_at = ? WHERE key = 'design_config'",
            (json.dumps(tr_cfg, ensure_ascii=False), json.dumps(en_cfg, ensure_ascii=False), now_iso),
        )
    conn.commit()
    print(f"  Repaired missing cover images={repaired_imgs}; updated bilingual design_config")

    # Regenerate storage/seed/live_msi_archive.json
    print("[6/6] Regenerating storage/seed/live_msi_archive.json & checkpointing WAL ...")
    cats_out = [dict(r) for r in cur.execute("SELECT slug, parent_slug, name_tr, name_en, group_type, description_tr, description_en, icon_class, sort_order, is_dynamic FROM categories ORDER BY sort_order, id")]
    issues_out = [dict(r) for r in cur.execute("SELECT issue_number, slug, title_tr, title_en, publish_month_tr, publish_month_en, year, cover_image, is_special_issue, event_name, page_count, dossier_count, summary_tr, summary_en, pdf_path, spreads_json FROM magazine_issues ORDER BY issue_number")]

    pls_map: dict[int, list[str]] = {}
    for r in cur.execute("SELECT content_id, category_slug FROM content_placements"):
        pls_map.setdefault(r["content_id"], []).append(r["category_slug"])
    ssb_map: dict[int, list[str]] = {}
    for r in cur.execute("SELECT content_id, ssb_code FROM content_ssb_tags"):
        ssb_map.setdefault(r["content_id"], []).append(r["ssb_code"])

    contents_out = []
    for r in cur.execute("SELECT * FROM contents ORDER BY id"):
        d = dict(r)
        d["placements"] = pls_map.get(d["id"], [])
        d["ssb_tags"] = ssb_map.get(d["id"], [])
        contents_out.append(d)

    redirects_out = [dict(r) for r in cur.execute("SELECT old_path, new_path, status_code FROM seo_redirects ORDER BY id")]
    archive_doc = {
        "exported_at": datetime.now(timezone.utc).isoformat(),
        "source": BASE_URL,
        "categories": cats_out,
        "issues": issues_out,
        "contents": contents_out,
        "seo_redirects": redirects_out,
    }
    ARCHIVE_JSON_PATH.parent.mkdir(parents=True, exist_ok=True)
    ARCHIVE_JSON_PATH.write_text(json.dumps(archive_doc, ensure_ascii=False), encoding="utf-8")

    cur.execute("PRAGMA wal_checkpoint(TRUNCATE);")
    total_tr = cur.execute("SELECT COUNT(*) FROM contents WHERE source_group != 'msi-tdr'").fetchone()[0]
    total_en = cur.execute("SELECT COUNT(*) FROM contents WHERE source_group = 'msi-tdr'").fetchone()[0]
    total_all = cur.execute("SELECT COUNT(*) FROM contents").fetchone()[0]
    conn.close()
    print(f"SYNC COMPLETE: total contents={total_all} (TR/bilingual={total_tr}, EN MSI TDR={total_en}), archive={ARCHIVE_JSON_PATH}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
