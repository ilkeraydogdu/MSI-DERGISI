"use strict";

/*======== Document Ready Function =========*/
jQuery(document).ready(function ($) {

    /**
     * Slicknav - Mobile Menu
     */
    $('.responsive-menu').slicknav({
        duration: 500,
        easingOpen: 'easeInExpo',
        easingClose: 'easeOutExpo',
        closedSymbol: '<i class="fa fa-plus"></i>',
        openedSymbol: '<i class="fa fa-minus"></i>',
        prependTo: '#slicknav-mobile',
        allowParentLinks: true,
        label: ""
    });

    $('#responsive-menu').slicknav({
        duration: 500,
        easingOpen: 'easeInExpo',
        easingClose: 'easeOutExpo',
        closedSymbol: '<i class="fa fa-plus"></i>',
        openedSymbol: '<i class="fa fa-minus"></i>',
        prependTo: '#slicknav-mobile',
        allowParentLinks: true,
        label: ""
    });

    /**
     * Rock-Solid Sticky Header (Always visible when scrolling down or up)
     */
    var cachedHeaderHeight = 0;
    function handleStickyNavbar() {
        var scrollVal = $(window).scrollTop();
        var $headerMenu = $('.header_menu');
        var $headerArea = $('.main_header_area');

        if (!cachedHeaderHeight && $headerMenu.length && !$headerMenu.hasClass('fixed-top')) {
            cachedHeaderHeight = $headerMenu.outerHeight() || 72;
        }

        if (scrollVal > 50) {
            if (!$headerMenu.hasClass('fixed-top')) {
                $headerArea.css('min-height', (cachedHeaderHeight || 72) + 'px');
                $headerMenu.addClass('fixed-top navbar-sticky-in');
                $('.navbar').addClass('navbar-sticky-in');
            }
        } else {
            if ($headerMenu.hasClass('fixed-top')) {
                $headerArea.css('min-height', '');
                $headerMenu.removeClass('fixed-top navbar-sticky-in');
                $('.navbar').removeClass('navbar-sticky-in');
            }
        }
    }

    $(window).on('scroll resize', handleStickyNavbar);
    handleStickyNavbar(); // Run on init in case page is refreshed while scrolled

    /**
     * Main Menu Slide Down Effect
     */
    var selected = $('#navbar li');
    selected.on("mouseenter", function () {
        $(this).find('ul').first().stop(true, true).delay(350).slideDown(500, 'easeInOutQuad');
    });

    selected.on("mouseleave", function () {
        $(this).find('ul').first().stop(true, true).delay(100).slideUp(150, 'easeInOutQuad');
    });

    /**
     * MSI Tactical Search Console Controls
     */
    $(document).on('click', 'a[href="#search1"], .mt_search', function (e) {
        e.preventDefault();
        var $search = $('#search1');
        $search.addClass('open');
        setTimeout(function () {
            $search.find('input[type="search"]').focus();
        }, 100);
    });

    $(document).on('click', '#search1 .close, #search1 .msi-search-close', function (e) {
        e.preventDefault();
        $('#search1').removeClass('open');
    });

    $(document).on('click', '#search1', function (e) {
        if ($(e.target).is('#search1')) {
            $('#search1').removeClass('open');
        }
    });

    $(document).on('keyup', function (e) {
        if (e.key === "Escape" || e.keyCode === 27) {
            $('#search1').removeClass('open');
        }
    });

    $(document).on('click', '#search1 .search-tag', function (e) {
        var href = $(this).attr('href');
        if (!href || href === '#' || href.indexOf('post-list') === -1) {
            e.preventDefault();
            var tagText = $(this).text().trim();
            var $input = $('#search1 input[type="search"]');
            if ($input.length) {
                $input.val(tagText).focus();
            }
        }
    });

});
