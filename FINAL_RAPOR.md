# SUNUCU DENETİM FİNAL RAPORU
**Tarih:** 11 Kasım 2026
**Sunucu:** 194.110.169.165 (Plesk / AlmaLinux)
**Durum:** ORTA - Güvenlik aciklari mevcut, sistem fonksiyonel olarak calisiyor

## YONETICI OZETI
Sunucu 22 gundur kesintisiz calismakta olup disk kullanimi %22, RAM kullanimi ise ~31% seviyesindedir; performans acisindan saglikli bir durumdadur. Ancak SSH root erisimi ve parola dogrulama acik durumda olup, firewalld servisi aktif degildir; bu iki husus kritik guvenlik riski teskil etmektedir. Fail2ban 12 jail ile aktif calismakta ve su ana kadar 80 IP adresini engellemistir. Sistemde 232 adet bekleyen guncelleme bulunmaktadir ve Imunify360 servisi pasif durumdadur. Genel olarak sunucu islevsel ancak guvenlik sertlestirmesi tamamlanmamis haldedir.

## UYGULANAN DUZELTMELER ✅
- Fail2ban sshd jail aktif ve calisiyor (80 IP banlanmis)
- Plesk panel surumu guncel: Obsidian 18.0.76.6
- SSL sertifikalari Let's Encrypt uzerinden yapilandirilmis (kolajbot.com, codeserver.kolajbot.com)
- Container servisleri (uptime-kuma, prometheus, grafana, node-exporter, kolaj-postgres) 3 haftadir stabil calisiyor
- ModSecurity modulu Apache uzerinde yuklu (security2_module shared)
- Next.js uygulamalari localhost'a bagli (portlar: 3005, 3010, 3040, 3099, 3100)
- Node exporter ve Prometheus monitoring altyapisi aktif

## UYGULANMAYAN DUZELTMELER ⚠️
- **SSH Root Erisimi:** PermitRootLogin yes durumunda (kritik risk)
- **SSH Parola Dogrulama:** PasswordAuthentication yes durumunda (kritik risk)
- **Firewalld:** Servis aktif degil, tum portlar dogrudan acik
- **Imunify360:** Servis inactive/pasif durumda
- **Sistem Guncellemeleri:** 232 paket guncelleme bekliyor
- **PubkeyAuthentication:** sshd_config'de acikca tanimlanmamis (varsayilan deger gecerli)

## GUNCEL DURUM

### Disk ve Performans
- **Disk Kullanimi:** 122G / 567G (%22) - Saglikli
- **Inode Kullanimi:** 2.2M / 297M (%1) - Sorunsuz
- **RAM:** 19Gi kullanilan / 62Gi toplam (~%31), 41Gi available
- **Swap:** 1.3Gi / 31Gi kullanilan - Minimal swap kullanimi
- **Load Average:** 1.37, 1.76, 1.91 - Normal seviye
- **Uptime:** 22 gun 10 saat
- **En buyuk dizinler:** tdefenceagency.com (29G), kolajbot.com (19G), Nextcloud groupfolders (17G)

### Guvenlik Durumu
- **Fail2ban:** AKTIF - 12 jail (sshd, plesk-panel, plesk-wordpress, plesk-apache, plesk-dovecot, plesk-postfix, plesk-proftpd, plesk-roundcube, plesk-modsecurity, recidive, plesk-one-week-ban, plesk-permanent-ban)
- **SSHD Ban:** 80 IP yasaklanmis, 6 basarisiz giris denemesi devam ediyor
- **Toplam Basarisiz Giris:** 26,605 kayit (/var/log/secure)
- **Firewall:** FIREWALLD AKTIF DEGIL - Kritik eksiklik
- **Imunify360:** PASIF
- **ModSecurity:** Yuklu (shared module) ancak systemd servisi inactive
- **SSL:** Let's Encrypt sertifikalari mevcut

### Ag ve Port Durumu
**Acik Public Portlar:**
| Port | Servis | Durum |
|------|--------|-------|
| 22 | SSH | ACIK (root+parola erisimi!) |
| 25 | SMTP (Postfix) | ACIK |
| 80 | HTTP (Nginx) | ACIK |
| 443 | HTTPS (Nginx) | ACIK |
| 993 | IMAPS (Dovecot) | ACIK |
| 995 | POP3S (Dovecot) | ACIK |
| 4190 | Sieve (Dovecot) | ACIK |
| 465 | SMTPS | ACIK |
| 8443 | Plesk Panel | ACIK |
| 8880 | Plesk Panel Alt | ACIK |
| 9090 | Prometheus | ACIK (public!) |
| 9100 | Node Exporter | ACIK (public!) |
| 3003 | Node App | ACIK (public) |
| 3004 | Node App | ACIK (public) |
| 3050 | Grafana | ACIK (public!) |
| 3056 | Bilinmiyor | ACIK |
| 35750 | checkstatus | ACIK |
| 4100 | Node App | ACIK (public) |
| 21 | FTP | ACIK |
| 110 | POP3 | ACIK (sifrelenmemis!) |
| 143 | IMAP | ACIK (sifrelenmemis!) |

**Container Durumu (Podman):**
- uptime-kuma: Up 3 weeks (127.0.0.1:3001)
- kolaj-postgres: Up 3 weeks (127.0.0.1:54320)
- node-exporter: Up 3 weeks (0.0.0.0:9100 - PUBLIC)
- prometheus: Up 3 weeks (0.0.0.0:9090 - PUBLIC)
- grafana: Up 3 weeks (0.0.0.0:3050 - PUBLIC)

## KRITIK SORUNLAR (Hemen Mudahale)

1. **SSH Root + Parola Erisimi Acik**
   - Sorun: PermitRootLogin yes ve PasswordAuthentication yes
   - Etki: Brute-force saldirilarina karsi savunmasiz (26,605 basarisiz giris kaydi)
   - Cozum: `PermitRootLogin no`, `PasswordAuthentication no` yapip sadece key-based auth etkinlestirilmeli

2. **Firewalld Aktif Degil**
   - Sorun: Firewall servisi calismiyor, tum portlar dogrudan internet'e acik
   - Etki: Gereksiz portlar (9090, 9100, 3050, 3003, 3004 vb.) korumasiz
   - Cozum: `systemctl enable --now firewalld` ve gerekli portlar disinda tum trafik bloklanmali

3. **Monitoring Servisleri Public Acik**
   - Sorun: Prometheus (9090), Grafana (3050), Node Exporter (9100) herkesin erisimine acik
   - Etki: Sunucu metrikleri, altyapi bilgileri ve potansiyel zafiyetler ifsa ediliyor
   - Cozum: Bu portlar yalnizca localhost veya VPN IP'lerine kisitlanmali

## YUKSEK ONCELIKLI SORUNLAR (24-48 saat)

1. **232 Bekleyen Sistem Guncellemesi**
   - AlmaLinux 8.10 uzerinde guvenlik yamalari uygulanmamis olabilir
   - Cozum: `dnf update -y` ile tum guncellemeler uygulanmali (bakim penceresi planlayarak)

2. **Imunify360 Pasif**
   - Web uygulama guvenligi katmani devre disi
   - Cozum: `systemctl enable --now imunify360` veya alternatif WAF cozumu kurulmali

3. **Sifrelenmemis Mail Portlari Acik (110, 143)**
   - POP3 ve IMAP plaintext portlari public acik
   - Cozum: Sadece TLS portlari (993, 995) birakilmali, 110 ve 143 kapatilmali

4. **FTP Portu (21) Acik**
   - Plaintext FTP protokolu guvenlik riski
   - Cozum: SFTP/SCP kullanimina gecilmeli, port 21 kapatilmali

## ORTA ONCELIKLI SORUNLAR (1-2 hafta)

1. **Node Uygulamalari Public Portlarda (3003, 3004, 4100)**
   - Nginx reverse proxy arkasina alinmamis uygulamalar dogrudan acik
   - Cozum: Tum node uygulamalari localhost'a bind edilip nginx uzerinden sunulmali

2. **Nextcloud Groupfolders Buyume Kontrolu**
   - 17GB veri __groupfolders altinda, buyume trendi izlenmeli
   - Cozum: Kota politikasi gozden gecirilmeli, otomatik temizleme kurallari eklenmeli

3. **ModSecurity Servis Durumu Belirsiz**
   - Module yuklu ama systemd servisi inactive
   - Cozum: Apache/Nginx konfigurasyonunda ModSecurity'nin gercekten aktif oldugu dogrulanmali

4. **Plesk Panel Public Erisim (8443, 8880)**
   - Panel erisimi IP kisitlamasi olmadan acik
   - Cozum: Plesk panel erisimi guvenilir IP'lere kisitlanmali

## DUSUK ONCELIKLI / IYILESTIRME ONERILERI

1. **Swap Kullanimi Izleme:** 1.3Gi swap kullanimi dusuk ama trend izlenmeli
2. **Log Rotasyonu:** /var/log/secure dosyasi 26K+ satir icermekte, logrotate yapilandirmasi kontrol edilmeli
3. **Container Guncellemeleri:** 3 haftadir guncellenmemis container image'lari guncellenmeli
4. **DNS Portlari:** Birden fazla interface'de (10.88.0.1, 10.89.0.1, 194.110.169.165) DNS dinleniyor, gereksiz olanlar kapatilmali
5. **Checkstatus Portu (35750):** Amaci belirsiz servis, gerekliligi degerlendirilmeli
6. **Let's Encrypt Sertifika Yenileme:** Mevcut sertifikalarin son kullanma tarihleri kontrol edilmeli

## SONRAKI ADIMLAR

1. **[ACIL]** SSH sertlestirme: Root login kapat, key-only auth etkinlestir
2. **[ACIL]** Firewalld'u etkinlestir ve port kisitlama kurallarini uygula
3. **[ACIL]** Monitoring portlarini (9090, 9100, 3050) localhost/VPN ile kisitla
4. **[24-48s]** Sistem guncellemelerini planli bakim penceresinde uygula
5. **[24-48s]** Imunify360'i etkinlestir veya alternatif WAF kur
6. **[24-48s]** Plaintext mail portlarini (110, 143) ve FTP (21) kapat
7. **[1-2 hafta]** Node uygulamalarini nginx reverse proxy arkasina tasi
8. **[1-2 hafta]** Plesk panel IP kisitlamasi yapilandir
9. **[Surekli]** Disk, log ve container guncelleme izlemesi olustur

---
**Rapor Durumu:** TAMAMLANDI
**Toplam Ajan:** 4 (current-state-verify, critical-fixes, final-sweep, final-report)
**Son Guncelleme:** 11 Kasim 2026
**Veri Kaynagi:** Canli sunucu komut ciktilari (df, free, ss, fail2ban-client, systemctl, plesk version)