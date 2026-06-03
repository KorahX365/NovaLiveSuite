let socket = null;
let reconnectTimer = null;
const alertQueue = [];
let isAlertActive = false;

// DOM Elements
const cardContainer = document.getElementById("alert-card-container");
const canvas = document.getElementById("particle-canvas");
const ctx = canvas.getContext("2d");

// Resize canvas
function resizeCanvas() {
  canvas.width = window.innerWidth;
  canvas.height = window.innerHeight;
}
window.addEventListener("resize", resizeCanvas);
resizeCanvas();

// Particle System
const particles = [];

class Particle {
  constructor(x, y, color) {
    this.x = x;
    this.y = y;
    const angle = Math.random() * Math.PI * 2;
    const speed = 2 + Math.random() * 6;
    this.vx = Math.cos(angle) * speed;
    this.vy = Math.sin(angle) * speed - 2; // Initial slight upward boost
    this.size = 3 + Math.random() * 5;
    this.alpha = 1;
    this.decay = 0.012 + Math.random() * 0.018;
    this.color = color;
    this.gravity = 0.06;
  }
  
  update() {
    this.x += this.vx;
    this.y += this.vy;
    this.vy += this.gravity;
    this.alpha -= this.decay;
  }
  
  draw() {
    ctx.save();
    ctx.globalAlpha = Math.max(0, this.alpha);
    ctx.fillStyle = this.color;
    ctx.shadowBlur = 10;
    ctx.shadowColor = this.color;
    ctx.beginPath();
    ctx.arc(this.x, this.y, this.size, 0, Math.PI * 2);
    ctx.fill();
    ctx.restore();
  }
}

// Particle Loop
let animationFrameId = null;
function updateParticles() {
  ctx.clearRect(0, 0, canvas.width, canvas.height);
  
  for (let i = particles.length - 1; i >= 0; i--) {
    const p = particles[i];
    p.update();
    p.draw();
    if (p.alpha <= 0) {
      particles.splice(i, 1);
    }
  }
  
  if (particles.length > 0) {
    animationFrameId = requestAnimationFrame(updateParticles);
  } else {
    animationFrameId = null;
    ctx.clearRect(0, 0, canvas.width, canvas.height);
  }
}

function triggerParticleExplosion(type) {
  const centerX = canvas.width / 2;
  const centerY = canvas.height / 2;
  
  let colors = [];
  if (type === "follow") {
    colors = ["#10b981", "#34d399", "#6ee7b7", "#a7f3d0"];
  } else if (type === "sub") {
    colors = ["#a855f7", "#c084fc", "#d8b4fe", "#f3e8ff"];
  } else { // donation
    colors = ["#f59e0b", "#fbbf24", "#fde047", "#fef3c7"];
  }
  
  // Create 60 particles
  for (let i = 0; i < 60; i++) {
    const color = colors[Math.floor(Math.random() * colors.length)];
    particles.push(new Particle(centerX, centerY, color));
  }
  
  if (!animationFrameId) {
    updateParticles();
  }
}

// WebSocket Connection
function connect() {
  const host = window.location.host || "localhost:777";
  const wsUrl = `ws://${host}/obs/nowplaying/ws`;
  
  socket = new WebSocket(wsUrl);
  
  socket.onopen = () => {
    console.log("Alerts overlay connected to WebSocket");
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
      // Look for custom alert events
      if (data.event === "alert") {
        queueAlert(data.data);
      }
    } catch (e) {
      console.error("Error parsing WebSocket alert message:", e);
    }
  };
  
  socket.onclose = () => {
    console.log("WebSocket connection lost. Reconnecting...");
    reconnectTimer = setTimeout(connect, 3000);
  };
  
  socket.onerror = (err) => {
    console.error("WebSocket error:", err);
  };
}

let config = null;

async function fetchConfig() {
  try {
    const res = await fetch("/api/config");
    config = await res.json();
  } catch (e) {
    console.error("Failed to load config", e);
  }
}

function hexToRgba(hex, alpha) {
  if (!hex) return `rgba(30, 30, 46, ${alpha})`;
  hex = hex.replace('#', '');
  if (hex.length === 3) {
    hex = hex.split('').map(char => char + char).join('');
  }
  const r = parseInt(hex.substring(0, 2), 16) || 0;
  const g = parseInt(hex.substring(2, 4), 16) || 0;
  const b = parseInt(hex.substring(4, 6), 16) || 0;
  return `rgba(${r}, ${g}, ${b}, ${alpha})`;
}

function queueAlert(alertData) {
  // Always fetch config on new alert to ensure it's fresh
  fetchConfig().then(() => {
    alertQueue.push(alertData);
    if (!isAlertActive) {
      processNextAlert();
    }
  });
}

function processNextAlert() {
  if (alertQueue.length === 0) {
    isAlertActive = false;
    return;
  }
  
  isAlertActive = true;
  const alert = alertQueue.shift();
  showAlert(alert);
}

function showAlert(alert) {
  // Clear previous cards
  cardContainer.innerHTML = "";
  
  const card = document.createElement("div");
  card.className = `alert-card ${alert.type}`;
  
  let titleText = "";
  let messageText = "";
  let imageUrl = "";
  let soundUrl = "";
  let duration = 5000;
  
  if (alert.type === "follow") {
    titleText = "New Follower!";
    messageText = "Welcome to the community!";
  } else if (alert.type === "sub") {
    titleText = "New Subscription!";
    const tier = alert.value || "Tier 1";
    messageText = `Subscribed at ${tier}!`;
  } else if (alert.type === "donation" || alert.type === "raid") {
    titleText = "New Raid!";
    const viewers = alert.value || "0";
    messageText = `Raiding with ${viewers} viewers!`;
  } else {
    titleText = alert.type.toUpperCase();
    messageText = alert.value || "";
  }

  // Load from configuration settings if available
  let layout = "top";
  let size = "80px";

  if (config && config.tools && config.tools.alerts && config.tools.alerts.settings) {
    const settings = config.tools.alerts.settings;
    layout = settings.layout || "top";
    
    if (settings.image_size) {
      size = settings.image_size.trim();
      if (!isNaN(size)) {
        size = size + "px";
      }
    }
    
    // Apply styling to card
    card.style.fontFamily = settings.font_family || "inherit";
    if (settings.font_size) card.style.fontSize = settings.font_size;
    if (settings.font_color) card.style.color = settings.font_color;
    
    const bgOpacity = settings.bg_opacity !== undefined ? settings.bg_opacity : 0.9;
    card.style.backgroundColor = hexToRgba(settings.bg_color, bgOpacity);
    
    if (settings.border_color) card.style.borderColor = settings.border_color;
    if (settings.border_width) {
      card.style.borderWidth = settings.border_width;
      card.style.borderStyle = "solid";
    }
    if (settings.border_radius) card.style.borderRadius = settings.border_radius;

    // Type specific config
    if (alert.type === "follow") {
      imageUrl = settings.follow_image;
      soundUrl = settings.follow_sound;
      duration = settings.follow_duration_ms || 5000;
      if (settings.follow_template) {
        messageText = settings.follow_template.replace("{user}", alert.name).replace("{name}", alert.name);
      }
    } else if (alert.type === "sub") {
      imageUrl = settings.sub_image;
      soundUrl = settings.sub_sound;
      duration = settings.sub_duration_ms || 5000;
      if (settings.sub_template) {
        messageText = settings.sub_template.replace("{user}", alert.name).replace("{name}", alert.name).replace("{count}", alert.value || "");
      }
    } else if (alert.type === "raid" || alert.type === "donation") {
      imageUrl = settings.raid_image;
      soundUrl = settings.raid_sound;
      duration = settings.raid_duration_ms || 5000;
      if (settings.raid_template) {
        messageText = settings.raid_template.replace("{user}", alert.name).replace("{name}", alert.name).replace("{count}", alert.value || "");
      }
    }
  }

  // Set card flex details based on layout
  if (layout === "left") {
    card.style.display = "flex";
    card.style.flexDirection = "row";
    card.style.alignItems = "center";
    card.style.textAlign = "left";
    card.style.gap = "16px";
  } else if (layout === "right") {
    card.style.display = "flex";
    card.style.flexDirection = "row-reverse";
    card.style.alignItems = "center";
    card.style.textAlign = "left";
    card.style.gap = "16px";
  } else if (layout === "over") {
    card.style.position = "relative";
    card.style.display = "flex";
    card.style.flexDirection = "column";
    card.style.justifyContent = "center";
    card.style.alignItems = "center";
    card.style.textAlign = "center";
    card.style.overflow = "hidden";
    
    // Match exact Now Playing card aspect ratio and dimensions
    card.style.width = "95vw";
    card.style.maxWidth = "960px";
    card.style.height = "28vw";
    card.style.maxHeight = "280px";
    card.style.fontSize = "clamp(12px, 2.7vw, 25px)";
    card.style.borderRadius = "1.3em";
    
    if (imageUrl) {
      card.style.backgroundImage = `linear-gradient(rgba(15, 15, 20, 0.65), rgba(15, 15, 20, 0.65)), url("${imageUrl}")`;
      card.style.backgroundSize = "cover";
      card.style.backgroundPosition = "center";
    }
  } else { // top
    card.style.display = "flex";
    card.style.flexDirection = "column";
    card.style.alignItems = "center";
    card.style.textAlign = "center";
    card.style.gap = "8px";
  }
  
  if (layout === "over") {
    card.innerHTML = `
      <div class="alert-content" style="width: 100%; text-shadow: 0 2px 8px rgba(0,0,0,0.9), 0 1px 3px rgba(0,0,0,0.9);">
        <div class="alert-title" style="font-weight: 800; font-size: 1.15em; margin-bottom: 6px; letter-spacing: 0.15em;">${titleText}</div>
        <div class="alert-name" style="font-weight: 800; font-size: 2.2em; color: #ffffff; margin-bottom: 6px;">${alert.name}</div>
        <div class="alert-message" style="opacity: 0.95; font-size: 1.05em; color: rgba(255, 255, 255, 0.9); font-weight: 500;">${messageText}</div>
      </div>
    `;
  } else {
    let imgHtml = "";
    if (imageUrl) {
      imgHtml = `<img class="alert-image" src="${imageUrl}" style="width: ${size}; height: ${size}; object-fit: contain; flex-shrink: 0;" />`;
    }
    card.innerHTML = `
      ${imgHtml}
      <div class="alert-content" style="flex: 1; width: 100%;">
        <div class="alert-title" style="font-weight: 800; font-size: 1.1em; margin-bottom: 4px;">${titleText}</div>
        <div class="alert-name" style="font-weight: 700; margin-bottom: 4px;">${alert.name}</div>
        <div class="alert-message" style="opacity: 0.9; font-size: 0.9em;">${messageText}</div>
      </div>
    `;
  }
  
  cardContainer.appendChild(card);
  
  // Play sound if configured
  if (soundUrl) {
    const audio = new Audio(soundUrl);
    audio.volume = 1.0;
    audio.play().catch(err => {
      console.warn("Failed to play audio (interaction limits may block autoplay on fresh page load):", err);
    });
  }
  
  // Trigger particle explosion right as card loads
  setTimeout(() => {
    triggerParticleExplosion(alert.type);
  }, 100);
  
  // Show card for duration, then fade out
  setTimeout(() => {
    card.classList.add("exit");
    // Wait for exit animation to complete (600ms) before transitioning to next
    setTimeout(() => {
      cardContainer.innerHTML = "";
      processNextAlert();
    }, 600);
  }, duration);
}

// Initial fetch and WebSocket connection
fetchConfig().then(() => {
  connect();
});
