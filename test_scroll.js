function smoothScrollTo(targetY, duration) {
  var startY = document.scrollingElement.scrollTop;
  var change = targetY - startY;
  var startTime = performance.now();

  function animateScroll(currentTime) {
    var elapsed = currentTime - startTime;
    var progress = Math.min(elapsed / duration, 1);
    
    // easeInOutQuad
    var ease = progress < 0.5 ? 2 * progress * progress : -1 + (4 - 2 * progress) * progress;
    
    document.scrollingElement.scrollTop = startY + change * ease;
    
    if (progress < 1) {
      requestAnimationFrame(animateScroll);
    }
  }
  
  requestAnimationFrame(animateScroll);
}
