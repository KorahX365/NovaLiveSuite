let socket = null;
let reconnectTimer = null;
let progressTimer = null;
let visualizerFrameId = null;

// Track state & config
let trackState = {
  title: "",
  artist: "",
  album: "",
  playback_status: "stopped",
  position: 0,
  duration: 0,
  source: "Windows Media",
  thumbnail: "",
  source_app_id: "",
  lyrics: []
};

let featuresConfig = {
  vinyl_enabled: true,
  lyrics_enabled: true,
  visualizer_enabled: true
};

let stylingConfig = {
  theme: "glassmorphic",
  accent_color: "auto"
};

// DOM elements
const container = document.getElementById("overlay-container");
const card = document.getElementById("card");
const artwork = document.getElementById("artwork");
const vinylArtwork = document.getElementById("vinyl-artwork");
const blurBg = document.getElementById("blur-bg");
const title = document.getElementById("title");
const artist = document.getElementById("artist");
const album = document.getElementById("album");
const progressFill = document.getElementById("progress-fill");
const timeCurrent = document.getElementById("time-current");
const timeTotal = document.getElementById("time-total");
const sourceText = document.getElementById("source-text");
const lyricsBox = document.getElementById("lyrics-box");
const lyricText = document.getElementById("lyric-text");
const canvas = document.getElementById("visualizer-canvas");
const ctx = canvas ? canvas.getContext("2d") : null;

// Set canvas dimensions
function resizeCanvas() {
  if (canvas) {
    canvas.width = canvas.offsetWidth;
    canvas.height = canvas.offsetHeight;
  }
}
window.addEventListener("resize", resizeCanvas);

// Connect WebSocket
function connect() {
  const host = window.location.host || "localhost:777";
  const wsUrl = `ws://${host}/obs/nowplaying/ws`;
  
  socket = new WebSocket(wsUrl);
  
  socket.onopen = () => {
    console.log("Connected to playback server");
    if (reconnectTimer) {
      clearTimeout(reconnectTimer);
      reconnectTimer = null;
    }
  };
  
  socket.onmessage = (event) => {
    try {
      const data = JSON.parse(event.data);
      if (data.event === "config_update") {
        window.location.reload();
        return;
      }
      if (data.config) {
        featuresConfig = data.config.features || featuresConfig;
        stylingConfig = data.config.styling || stylingConfig;
        applyConfig();
      }
      if (data.state) {
        updateState(data.state);
      }
    } catch (e) {
      console.error("Error parsing WebSocket message:", e);
    }
  };
  
  socket.onclose = () => {
    console.log("Connection closed. Reconnecting...");
    cleanup();
    reconnectTimer = setTimeout(connect, 3000);
  };
  
  socket.onerror = (err) => {
    console.error("WebSocket error:", err);
  };
}

function cleanup() {
  clearInterval(progressTimer);
  container.className = "stopped";
  if (visualizerFrameId) {
    cancelAnimationFrame(visualizerFrameId);
    visualizerFrameId = null;
  }
}

// Format duration
function formatTime(seconds) {
  if (isNaN(seconds) || seconds < 0) return "0:00";
  const mins = Math.floor(seconds / 60);
  const secs = Math.floor(seconds % 60);
  return `${mins}:${secs.toString().padStart(2, '0')}`;
}

// Dynamic abstract cover when none exists
function generatePlaceholder(txt1, txt2) {
  const pc = document.createElement("canvas");
  pc.width = 300;
  pc.height = 300;
  const pctx = pc.getContext("2d");
  
  const hash = ((txt1 || "").length * 31 + (txt2 || "").length) % 360;
  const hue1 = hash;
  const hue2 = (hash + 120) % 360;
  
  const grad = pctx.createLinearGradient(0, 0, 300, 300);
  grad.addColorStop(0, `hsl(${hue1}, 75%, 35%)`);
  grad.addColorStop(0.5, `hsl(${(hue1 + 60) % 360}, 85%, 45%)`);
  grad.addColorStop(1, `hsl(${hue2}, 90%, 25%)`);
  pctx.fillStyle = grad;
  pctx.fillRect(0, 0, 300, 300);
  
  pctx.fillStyle = "rgba(255, 255, 255, 0.05)";
  pctx.beginPath();
  pctx.arc(150, 150, 100, 0, Math.PI * 2);
  pctx.fill();
  
  pctx.fillStyle = "rgba(255, 255, 255, 0.7)";
  pctx.font = "bold 80px Outfit, sans-serif";
  pctx.textBaseline = "middle";
  pctx.textAlign = "center";
  
  const init1 = (txt1 || "N").charAt(0).toUpperCase();
  const init2 = (txt2 || "P").charAt(0).toUpperCase();
  pctx.fillText(init1 + init2, 150, 150);
  
  return pc.toDataURL();
}

// Extract dominant color from cover art
function extractColor(imgSrc) {
  if (stylingConfig.accent_color !== "auto") {
    // Default styling colors
    document.documentElement.style.setProperty("--accent-color", "#a855f7");
    document.documentElement.style.setProperty("--accent-color-rgb", "168, 85, 247");
    document.documentElement.style.setProperty("--accent-gradient", "linear-gradient(135deg, #a855f7 0%, #ec4899 100%)");
    document.documentElement.style.setProperty("--accent-tint", "rgb(241, 227, 253)");
    return;
  }
  const img = new Image();
  img.crossOrigin = "Anonymous";
  img.src = imgSrc;
  img.onload = () => {
    try {
      const canvasTmp = document.createElement("canvas");
      const ctxTmp = canvasTmp.getContext("2d");
      canvasTmp.width = 10;
      canvasTmp.height = 10;
      ctxTmp.drawImage(img, 0, 0, 10, 10);
      const data = ctxTmp.getImageData(0, 0, 10, 10).data;
      
      let r = 0, g = 0, b = 0, count = 0;
      for (let i = 0; i < data.length; i += 4) {
        const br = (data[i] * 299 + data[i+1] * 587 + data[i+2] * 114) / 1000;
        if (br > 40 && br < 220) {
          r += data[i]; g += data[i+1]; b += data[i+2];
          count++;
        }
      }
      
      if (count > 0) {
        r = Math.floor(r / count); g = Math.floor(g / count); b = Math.floor(b / count);
      } else {
        r = data[40]; g = data[41]; b = data[42];
      }
      
      // Ensure color is bright enough for clear contrast
      const brightness = (r * 299 + g * 587 + b * 114) / 1000;
      if (brightness < 130) {
        const boost = 130 - brightness;
        r = Math.min(255, Math.floor(r + boost * 0.8));
        g = Math.min(255, Math.floor(g + boost * 0.8));
        b = Math.min(255, Math.floor(b + boost * 0.8));
      }
      
      const colorStr = `rgb(${r}, ${g}, ${b})`;
      const gradientStr = `linear-gradient(135deg, rgb(${r}, ${g}, ${b}) 0%, rgb(${Math.max(0, r-45)}, ${Math.max(0, g-45)}, ${Math.max(0, b-45)}) 100%)`;
      
      const tintR = Math.floor(255 * 0.85 + r * 0.15);
      const tintG = Math.floor(255 * 0.85 + g * 0.15);
      const tintB = Math.floor(255 * 0.85 + b * 0.15);
      const tintColorStr = `rgb(${tintR}, ${tintG}, ${tintB})`;
      
      document.documentElement.style.setProperty("--accent-color", colorStr);
      document.documentElement.style.setProperty("--accent-color-rgb", `${r}, ${g}, ${b}`);
      document.documentElement.style.setProperty("--accent-gradient", gradientStr);
      document.documentElement.style.setProperty("--accent-tint", tintColorStr);
    } catch (e) {
      console.warn(e);
    }
  };
}

function hexToRgba(hex, opacity) {
  let c;
  if(/^#([A-Fa-f0-9]{3}){1,2}$/.test(hex)){
    c= hex.substring(1).split('');
    if(c.length== 3){
      c= [c[0], c[0], c[1], c[1], c[2], c[2]];
    }
    c= '0x' + c.join('');
    return 'rgba('+[(c>>16)&255, (c>>8)&255, c&255].join(',')+','+opacity+')';
  }
  return `rgba(15, 15, 20, ${opacity})`;
}

// Apply layout config
function applyConfig() {
  // Theme class
  container.classList.remove("theme-minimalist", "theme-glassmorphic");
  container.classList.add("theme-" + stylingConfig.theme);
  
  // Vinyl visibility class
  if (featuresConfig.vinyl_enabled) {
    container.classList.add("vinyl-active");
  } else {
    container.classList.remove("vinyl-active");
  }
  
  // Visualizer class
  if (featuresConfig.visualizer_enabled) {
    container.classList.add("visualizer-active");
    resizeCanvas();
    if (!visualizerFrameId) {
      startVisualizer();
    }
  } else {
    container.classList.remove("visualizer-active");
    if (visualizerFrameId) {
      cancelAnimationFrame(visualizerFrameId);
      visualizerFrameId = null;
    }
  }

  const formatPx = (val) => {
    if (val === undefined || val === null || val === "") return "";
    const s = val.toString().trim();
    return /^\d+$/.test(s) ? s + "px" : s;
  };

  // Custom styles application
  if (stylingConfig.font_family) {
    container.style.fontFamily = stylingConfig.font_family + ", sans-serif";
  }
  if (stylingConfig.font_size) {
    container.style.fontSize = formatPx(stylingConfig.font_size);
  }
  if (stylingConfig.color_title) {
    title.style.color = stylingConfig.color_title;
  }
  if (stylingConfig.color_artist) {
    artist.style.color = stylingConfig.color_artist;
  }
  if (stylingConfig.color_bg) {
    const opacity = stylingConfig.bg_opacity !== undefined ? parseFloat(stylingConfig.bg_opacity) : 0.5;
    card.style.backgroundColor = hexToRgba(stylingConfig.color_bg, opacity);
    card.style.background = hexToRgba(stylingConfig.color_bg, opacity);
  }
  if (stylingConfig.border_color) {
    card.style.borderColor = stylingConfig.border_color;
  }
  if (stylingConfig.border_width) {
    card.style.borderWidth = formatPx(stylingConfig.border_width);
    card.style.borderStyle = "solid";
  }
  if (stylingConfig.border_radius) {
    card.style.borderRadius = formatPx(stylingConfig.border_radius);
    const blurBg = document.getElementById("blur-bg");
    if (blurBg) blurBg.style.borderRadius = formatPx(stylingConfig.border_radius);
  }
  if (stylingConfig.padding_px) {
    const pad = Math.min(40, parseInt(stylingConfig.padding_px));
    card.style.padding = formatPx(pad);
  }
}

let hideTimer = null;

function checkTitleMarquee() {
  // Reset marquee
  title.classList.remove("marquee");
  title.parentElement.classList.remove("marquee-active");
  title.style.removeProperty("--scroll-dist");
  title.style.animationDuration = "";
  
  if (stylingConfig.scroll_text === false) {
    return;
  }
  
  const wrapperWidth = title.parentElement.clientWidth;
  const scrollWidth = title.scrollWidth;
  const scrollDist = scrollWidth - wrapperWidth;
  
  if (wrapperWidth > 0 && scrollDist > 0) {
    title.style.setProperty("--scroll-dist", `-${scrollDist}px`);
    title.classList.add("marquee");
    title.parentElement.classList.add("marquee-active");
    const speed = 25; // 25px per second
    const duration = Math.max(6, scrollDist / speed);
    title.style.animationDuration = `${duration}s`;
  }
}

// Update UI state
function updateState(newState) {
  const hasTrackChanged = newState.title !== trackState.title || newState.artist !== trackState.artist;
  
  trackState = newState;
  
  // Base playback status class
  container.classList.remove("playing", "paused", "stopped");
  container.classList.add(trackState.playback_status);
  
  // Handle auto-hide on pause/stop
  clearTimeout(hideTimer);
  container.classList.remove("paused-hidden");
  if (trackState.playback_status === "paused" || trackState.playback_status === "stopped") {
    hideTimer = setTimeout(() => {
      container.classList.add("paused-hidden");
    }, 3000); // 3 seconds
  }
  
  if (trackState.title) {
    // Add source class
    const sourceClass = "source-" + trackState.source.toLowerCase().replace(/\s+/g, '-');
    container.className = `${trackState.playback_status} theme-${stylingConfig.theme} ${featuresConfig.vinyl_enabled ? 'vinyl-active' : ''} ${featuresConfig.visualizer_enabled ? 'visualizer-active' : ''} ${sourceClass}`;
    
    // Maintain paused-hidden class if already timer triggered
    if (trackState.playback_status === "paused" && !container.classList.contains("playing") && container.classList.contains("paused-hidden")) {
      container.classList.add("paused-hidden");
    }
    
    // Maintain alert-hidden class if timer is active
    if (alertHideTimer) {
      container.classList.add("alert-hidden");
    }
    
    if (hasTrackChanged) {
      fadeOutTexts(() => {
        title.innerText = trackState.title;
        artist.innerText = trackState.artist;
        let albumText = "";
        if (trackState.year) {
          albumText += trackState.year;
        }
        if (trackState.album) {
          if (albumText) albumText += " • ";
          albumText += trackState.album;
        }
        album.innerText = albumText;
        sourceText.innerText = trackState.source;
        
        let artSrc = "";
        if (trackState.thumbnail) {
          artSrc = `data:image/jpeg;base64,${trackState.thumbnail}`;
        } else {
          artSrc = generatePlaceholder(trackState.title, trackState.artist);
        }
        
        artwork.src = artSrc;
        vinylArtwork.src = artSrc;
        blurBg.style.backgroundImage = `url('${artSrc}')`;
        extractColor(artSrc);
        
        fadeInTexts();
      });
    } else {
      sourceText.innerText = trackState.source;
      checkTitleMarquee();
    }
    
    updateTimeline();
    updateLyrics();
    
    clearInterval(progressTimer);
    if (trackState.playback_status === "playing") {
      progressTimer = setInterval(() => {
        if (trackState.duration > 0 && trackState.position < trackState.duration) {
          trackState.position += 0.25;
          updateTimeline();
          updateLyrics();
        }
      }, 250);
    }
  } else {
    container.className = "stopped";
    if (alertHideTimer) {
      container.classList.add("alert-hidden");
    }
    clearInterval(progressTimer);
  }
}

function fadeOutTexts(callback) {
  title.classList.add("fade-out");
  artist.classList.add("fade-out");
  album.classList.add("fade-out");
  setTimeout(callback, 250);
}

function fadeInTexts() {
  title.classList.remove("fade-out");
  artist.classList.remove("fade-out");
  album.classList.remove("fade-out");
  
  title.classList.add("fade-in");
  artist.classList.add("fade-in");
  album.classList.add("fade-in");
  
  setTimeout(() => {
    title.classList.remove("fade-in");
    artist.classList.remove("fade-in");
    album.classList.remove("fade-in");
    checkTitleMarquee();
  }, 300);
}

function updateTimeline() {
  const percent = trackState.duration > 0 ? (trackState.position / trackState.duration) * 100 : 0;
  progressFill.style.width = `${Math.min(100, Math.max(0, percent))}%`;
  
  timeCurrent.innerText = formatTime(trackState.position);
  timeTotal.innerText = formatTime(trackState.duration);
}

// Synced lyrics highlight logic
function updateLyrics() {
  const isNoLyricsRoute = window.location.pathname.includes("/nolyrics");
  if (isNoLyricsRoute || !featuresConfig.lyrics_enabled || !trackState.lyrics || trackState.lyrics.length === 0) {
    lyricsBox.classList.add("hidden");
    album.classList.remove("hidden");
    return;
  }
  
  lyricsBox.classList.remove("hidden");
  album.classList.add("hidden");
  
  // Find current active lyric line
  let activeText = "";
  const pos = trackState.position;
  
  for (let i = 0; i < trackState.lyrics.length; i++) {
    if (trackState.lyrics[i].time <= pos) {
      activeText = trackState.lyrics[i].text;
    } else {
      break;
    }
  }
  
  if (!activeText && trackState.lyrics.length > 0) {
    // If before first line, show upcoming
    activeText = "...";
  }
  if (lyricText.innerText !== activeText) {
    lyricText.style.animation = "none";
    lyricText.offsetHeight; // Trigger reflow
    lyricText.innerText = activeText;
    lyricText.style.animation = "slide-up 0.8s cubic-bezier(0.16, 1, 0.3, 1)";
  }
}

// Procedural Audio Visualizer Loop
let wavePhase = 0;
function startVisualizer() {
  function draw() {
    if (!featuresConfig.visualizer_enabled || !canvas || !ctx) {
      visualizerFrameId = null;
      return;
    }
    
    ctx.clearRect(0, 0, canvas.width, canvas.height);
    
    // Wave parameters based on play state
    const isPlaying = trackState.playback_status === "playing";
    const speed = isPlaying ? 0.08 : 0.015; // Slow breath when paused/stopped
    const maxAmp = isPlaying ? 16 : 4;       // Low ripple when paused
    
    wavePhase += speed;
    
    const width = canvas.width;
    const height = canvas.height;
    const accentColor = getComputedStyle(document.documentElement).getPropertyValue("--accent-color").trim() || "rgb(168, 85, 247)";
    
    // Draw 3 layers of overlapping waves for depth
    const layers = [
      { amp: maxAmp, freq: 0.008, phaseShift: 0, opacity: 0.15 },
      { amp: maxAmp * 0.7, freq: 0.014, phaseShift: Math.PI * 0.5, opacity: 0.3 },
      { amp: maxAmp * 0.4, freq: 0.02, phaseShift: Math.PI, opacity: 0.6 }
    ];
    
    layers.forEach(layer => {
      ctx.beginPath();
      ctx.strokeStyle = accentColor;
      ctx.lineWidth = layer.opacity === 0.6 ? 2 : 1.2;
      ctx.globalAlpha = layer.opacity;
      
      for (let x = 0; x <= width; x += 5) {
        // Multi-frequency wave formula
        const y = height/2 + 
                  Math.sin(x * layer.freq + wavePhase + layer.phaseShift) * layer.amp * 
                  Math.sin(x * 0.002 + wavePhase * 0.5); // Modulation envelope
                  
        if (x === 0) {
          ctx.moveTo(x, y);
        } else {
          ctx.lineTo(x, y);
        }
      }
      ctx.stroke();
    });
    
    ctx.globalAlpha = 1.0;
    visualizerFrameId = requestAnimationFrame(draw);
  }
  
  visualizerFrameId = requestAnimationFrame(draw);
}

// Start client connection
resizeCanvas();
connect();

// --- Hide on Alert logic ---
// Listen to SSE stream for twitch_alert events
let alertHideTimer = null;
(function connectAlertSSE() {
  const sse = new EventSource('/api/stream');

  sse.addEventListener('initial_config', (e) => {
    try {
      const data = JSON.parse(e.data);
      if (data.tools && data.tools.nowplaying && data.tools.nowplaying.settings) {
        featuresConfig.hide_on_alert = data.tools.nowplaying.settings.hide_on_alert === true;
      }
    } catch(_) {}
  });

  sse.addEventListener('config_update', (e) => {
    try {
      const data = JSON.parse(e.data);
      if (data.tools && data.tools.nowplaying && data.tools.nowplaying.settings) {
        featuresConfig.hide_on_alert = data.tools.nowplaying.settings.hide_on_alert === true;
      }
    } catch(_) {}
  });

  sse.addEventListener('twitch_alert', (e) => {
    if (!featuresConfig.hide_on_alert) return;
    try {
      // Force exactly 6500ms duration from when the alert fires
      const totalHideMs = 6500;

      // Hide overlay
      container.classList.add('alert-hidden');

      // Reset timer each time an alert fires (e.g. rapid consecutive alerts)
      if (alertHideTimer) clearTimeout(alertHideTimer);
      alertHideTimer = setTimeout(() => {
        container.classList.remove('alert-hidden');
        alertHideTimer = null;
      }, totalHideMs);
    } catch(_) {}
  });
})();
