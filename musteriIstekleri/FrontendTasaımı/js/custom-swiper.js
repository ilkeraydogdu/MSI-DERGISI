var interleaveOffset = 0.5;
var swiperOptions = {
    loop: true,
    speed: 1000,
    grabCursor: true,
    slidesPerView: 1,
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
        delay: 5000,
        disableOnInteraction: false,
    },
    on: {
        progress: function() {
            var swiper = this;
            for (var i = 0; i < swiper.slides.length; i++) {
                var slideProgress = swiper.slides[i].progress;
                var innerOffset = swiper.width * interleaveOffset;
                var innerTranslate = slideProgress * innerOffset;
                var slideInner = swiper.slides[i].querySelector(".slide-inner");
                if (slideInner) {
                    slideInner.style.transform = "translate3d(" + innerTranslate + "px, 0, 0)";
                }
            } 
        },
        touchStart: function() {
            var swiper = this;
            for (var i = 0; i < swiper.slides.length; i++) {
                var slideInner = swiper.slides[i].querySelector(".slide-inner");
                if (slideInner) slideInner.style.transition = "";
            }
        },
        setTransition: function(speed) {
            var swiper = this;
            for (var i = 0; i < swiper.slides.length; i++) {
                var slideInner = swiper.slides[i].querySelector(".slide-inner");
                if (slideInner) slideInner.style.transition = speed + "ms";
            }
        }
    }
};

var swiper = new Swiper(".swiper-container", swiperOptions);