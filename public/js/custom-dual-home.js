/**
 * MSI Dergisi - Dual Homepage Slider & Tactical Switcher
 * Mode A: Dergi Tanıtım Sitesi (Default, Switch Left)
 * Mode B: Savunma Haber Portalı (Switch Right)
 */
(function($) {
    'use strict';

    $(document).ready(function() {
        var $switch = $('#msi-tactical-switch');
        var $viewport = $('#msi-dual-home-viewport');
        var $track = $('#msi-dual-home-track');
        var $panelMag = $('#msi-panel-magazine');
        var $panelNews = $('#msi-panel-news');
        var $labelMag = $('#label-mag');
        var $labelNews = $('#label-news');
        var $modeLabel = $('#msi-current-mode-label');

        function updateViewportHeight() {
            var isNews = $('body').hasClass('show-news');
            var activePanel = isNews ? $panelNews : $panelMag;
            if (activePanel.length) {
                var h = activePanel.outerHeight();
                if (h > 0) {
                    $viewport.css('height', h + 'px');
                }
            }
        }

        function setMode(mode, doScroll) {
            if (mode === 'news') {
                $('body').addClass('show-news');
                $switch.addClass('active-news').attr('aria-checked', 'true');
                $labelMag.removeClass('active');
                $labelNews.addClass('active');
                $modeLabel.html('<strong class="text-white">SAVUNMA HABER MERKEZİ</strong> <span class="text-white-50 ms-2 small d-none d-md-inline">• Güncel Savunma Sanayii &amp; Teknoloji Gelişmeleri</span>');
                history.replaceState(null, null, '#haber');
                
                // Recalculate heights & trigger swiper/slick resize
                setTimeout(function() {
                    window.dispatchEvent(new Event('resize'));
                    updateViewportHeight();
                }, 150);
                setTimeout(function() {
                    window.dispatchEvent(new Event('resize'));
                    updateViewportHeight();
                }, 650);
            } else {
                $('body').removeClass('show-news');
                $switch.removeClass('active-news').attr('aria-checked', 'false');
                $labelNews.removeClass('active');
                $labelMag.addClass('active');
                $modeLabel.html('<strong class="text-white">DERGİ TANITIM MERKEZİ</strong> <span class="text-white-50 ms-2 small d-none d-md-inline">• Sayı 280 (Eylül 2026) Özel Lansmanı</span>');
                history.replaceState(null, null, '#dergi');
                
                setTimeout(function() {
                    updateViewportHeight();
                }, 150);
                setTimeout(function() {
                    updateViewportHeight();
                }, 650);
            }
            updateViewportHeight();

            if (doScroll) {
                var $switchBar = $('#msi-platform-switcher-bar');
                if ($switchBar.length) {
                    var switcherTop = $switchBar.offset().top - 70;
                    $('html, body').animate({ scrollTop: Math.max(0, switcherTop) }, 400);
                }
            }
        }

        // Click on Tactical Switch Widget
        $switch.on('click', function(e) {
            e.preventDefault();
            var isNews = $('body').hasClass('show-news');
            setMode(isNews ? 'mag' : 'news', false);
        });

        // Keyboard navigation for switch
        $switch.on('keydown', function(e) {
            if (e.key === 'Enter' || e.key === ' ') {
                e.preventDefault();
                var isNews = $('body').hasClass('show-news');
                setMode(isNews ? 'mag' : 'news', false);
            }
        });

        // Click on left/right side labels
        $labelMag.on('click', function(e) {
            e.preventDefault();
            setMode('mag', false);
        });

        $labelNews.on('click', function(e) {
            e.preventDefault();
            setMode('news', false);
        });

        // In-page bridge buttons
        $(document).on('click', '.btn-trigger-news-mode', function(e) {
            e.preventDefault();
            setMode('news', true);
        });

        $(document).on('click', '.btn-trigger-mag-mode', function(e) {
            e.preventDefault();
            setMode('mag', true);
        });

        // Check URL hash or server-rendered body.show-news / ?mode=news on page load
        if (window.location.hash === '#haber' || window.location.search.indexOf('mode=news') !== -1 || $('body').hasClass('show-news')) {
            setMode('news', false);
        } else {
            setMode('mag', false);
        }

        // Window resize and image loading height adjustments
        $(window).on('resize', function() {
            updateViewportHeight();
        });

        // Ensure proper height after images load
        $('img').on('load', function() {
            updateViewportHeight();
        });
        setTimeout(updateViewportHeight, 300);
        setTimeout(updateViewportHeight, 800);
        setTimeout(updateViewportHeight, 1500);
    });
})(jQuery);
