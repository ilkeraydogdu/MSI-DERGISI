/**
 * Consolidated Swiper configuration objects.
 * Replaces: custom-swiper1.js, custom-swiper2.js, custom-swiper3.js,
 *           custom-swiper4.js, custom-swiper5.js
 */

var interleaveOffset = 0.5;

/**
 * Horizontal interleave effect slider (from custom-swiper1.js)
 */
var horizontalInterleave = {
    loop: true,
    speed: 3000,
    grabCursor: true,
    slidesPerView: 3,
    watchSlidesProgress: true,
    mousewheelControl: true,
    keyboardControl: true,
    navigation: {
        nextEl: ".swiper-button-next",
        prevEl: ".swiper-button-prev"
    },
    pagination: {
        el: '.swiper-pagination',
        clickable: true,
    },
    autoplay: {
        delay: 3000,
    },
    fadeEffect: {
        crossFade: true
    },
    on: {
        progress: function() {
            var swiper = this;
            for (var i = 0; i < swiper.slides.length; i++) {
                var slideProgress = swiper.slides[i].progress;
                var innerOffset = swiper.width * interleaveOffset;
                var innerTranslate = slideProgress * innerOffset;
                swiper.slides[i].querySelector(".slide-inner").style.transform = "translate3d(" + innerTranslate + "px, 0, 0)";
            }
        },
        touchStart: function() {
            var swiper = this;
            for (var i = 0; i < swiper.slides.length; i++) {
                swiper.slides[i].style.transition = "";
            }
        },
        setTransition: function(speed) {
            var swiper = this;
            for (var i = 0; i < swiper.slides.length; i++) {
                swiper.slides[i].style.transition = speed + "ms";
                swiper.slides[i].querySelector(".slide-inner").style.transition = speed + "ms";
            }
        }
    }
};

/**
 * Vertical interleave slider (from custom-swiper2.js)
 */
var verticalInterleave = {
    loop: true,
    speed: 3000,
    direction: 'vertical',
    grabCursor: true,
    slidesPerView: 3,
    watchSlidesProgress: true,
    mousewheelControl: true,
    keyboardControl: true,
    pagination: {
        el: '.swiper-pagination',
        clickable: true,
    },
    autoplay: {
        delay: 3000,
    },
    fadeEffect: {
        crossFade: true
    }
};

/**
 * Centered responsive slider with breakpoints (from custom-swiper3.js / custom-swiper5.js)
 * Uses the richer breakpoint set from swiper3 (640/991/1024).
 * For the variant from swiper5 (640/768/1024), use centeredResponsiveAlt below.
 */
var centeredResponsive = {
    slidesPerView: 3,
    centeredSlides: true,
    spaceBetween: 15,
    loop: true,
    speed: 2000,
    grabCursor: true,
    watchSlidesProgress: true,
    mousewheelControl: true,
    keyboardControl: true,
    autoplay: {
        delay: 3000,
    },
    navigation: {
        nextEl: ".swiper-button-next",
        prevEl: ".swiper-button-prev"
    },
    pagination: {
        el: '.swiper-pagination',
        clickable: true,
    },
    breakpoints: {
        640: {
            slidesPerView: 1,
            spaceBetween: 0,
        },
        991: {
            slidesPerView: 1,
            spaceBetween: 0,
        },
        1024: {
            slidesPerView: 2,
            spaceBetween: 0,
        }
    }
};

/**
 * Alternative centered responsive variant (from custom-swiper5.js)
 * Breakpoints: 640/768/1024 with spaceBetween: 0
 */
var centeredResponsiveAlt = {
    slidesPerView: 3,
    centeredSlides: true,
    spaceBetween: 0,
    loop: true,
    speed: 2000,
    grabCursor: true,
    watchSlidesProgress: true,
    mousewheelControl: true,
    keyboardControl: true,
    autoplay: {
        delay: 3000,
    },
    navigation: {
        nextEl: ".swiper-button-next",
        prevEl: ".swiper-button-prev"
    },
    pagination: {
        el: '.swiper-pagination',
        clickable: true,
    },
    breakpoints: {
        640: {
            slidesPerView: 1,
            spaceBetween: 0,
        },
        768: {
            slidesPerView: 2,
            spaceBetween: 0,
        },
        1024: {
            slidesPerView: 2,
            spaceBetween: 0,
        }
    }
};

/**
 * Gallery thumbs configuration (from custom-swiper4.js)
 * Use galleryThumbsConfig for the thumbnail strip,
 * and galleryTopConfig for the main gallery slider.
 */
var galleryThumbsConfig = {
    spaceBetween: 0,
    slidesPerView: 4,
    loop: true,
    direction: 'vertical',
    freeMode: true,
    loopedSlides: 5,
    watchSlidesVisibility: true,
    watchSlidesProgress: true,
};

var galleryTopConfig = {
    spaceBetween: 0,
    loop: true,
    loopedSlides: 5,
    navigation: {
        nextEl: '.swiper-button-next',
        prevEl: '.swiper-button-prev',
    },
    pagination: {
        el: '.swiper-pagination',
        clickable: true,
    },
    speed: 3000,
    autoplay: {
        delay: 3000,
    },
    fadeEffect: {
        crossFade: true
    },
    effect: 'fade',
};