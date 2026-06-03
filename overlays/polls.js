let socket = null;
let reconnectTimer = null;
let pollTimerInterval = null;

// DOM Elements
const pollContainer = document.getElementById("poll-container");
const pollTitle = document.getElementById("poll-title");
const pollStatus = document.getElementById("poll-status");
const pollTimer = document.getElementById("poll-timer");
const pollTotalVotes = document.getElementById("poll-total-votes");
const timerFill = document.getElementById("timer-fill");
const optionsContainer = document.getElementById("poll-options-container");

// State
let activePoll = null;
let durationLeft = 0;
let totalDuration = 0;

// Configuración de WebSocket
function connect() {
    const host = window.location.host || "localhost:777";
    const wsUrl = `ws://${host}/obs/polls/ws`;
    
    socket = new WebSocket(wsUrl);
    
    socket.onopen = () => {
        console.log("Poll overlay connected to WebSocket");
        if (reconnectTimer) {
            clearTimeout(reconnectTimer);
            reconnectTimer = null;
        }
    };
    
    socket.onmessage = (event) => {
        try {
            const data = JSON.parse(event.data);
            
            if (data.event === "poll_begin") {
                startPoll(data.data);
            } else if (data.event === "poll_progress") {
                updatePoll(data.data);
            } else if (data.event === "poll_end") {
                endPoll(data.data);
            }
        } catch (e) {
            console.error("Error parsing WebSocket poll message:", e);
        }
    };
    
    socket.onclose = () => {
        console.log("WebSocket connection lost. Reconnecting in 3s...");
        reconnectTimer = setTimeout(connect, 3000);
        
        // Si perdemos conexión, después de unos segundos iniciamos la simulación demo para OBS
        if (!activePoll) {
            setTimeout(() => {
                if (socket.readyState !== WebSocket.OPEN && !activePoll) {
                    startDemoSimulation();
                }
            }, 2000);
        }
    };
    
    socket.onerror = (err) => {
        console.error("WebSocket error:", err);
    };
}

// Iniciar una encuesta
function startPoll(pollData) {
    activePoll = pollData;
    
    // Configurar metadatos
    pollTitle.textContent = pollData.title;
    pollStatus.textContent = "ENCUESTA ACTIVA";
    pollStatus.className = "badge active-badge";
    pollTotalVotes.textContent = `${pollData.total_votes || 0} votos`;
    
    // Configurar tiempo
    totalDuration = pollData.duration_seconds || 60;
    durationLeft = totalDuration;
    updateTimerUI();
    
    // Iniciar el intervalo del temporizador
    if (pollTimerInterval) clearInterval(pollTimerInterval);
    pollTimerInterval = setInterval(() => {
        durationLeft--;
        if (durationLeft <= 0) {
            durationLeft = 0;
            clearInterval(pollTimerInterval);
        }
        updateTimerUI();
    }, 1000);
    
    // Renderizar opciones
    renderOptions(pollData.options);
    
    // Mostrar overlay
    pollContainer.classList.remove("hidden");
}

// Actualizar en vivo los votos de la encuesta
function updatePoll(pollData) {
    if (!activePoll) return;
    
    activePoll.total_votes = pollData.total_votes;
    pollTotalVotes.textContent = `${pollData.total_votes} votos`;
    
    // Calcular opción ganadora temporal
    const options = calculatePercentages(pollData.options, pollData.total_votes);
    
    // Actualizar barras de progreso y textos en la UI
    options.forEach(opt => {
        const row = document.getElementById(`opt-${opt.id}`);
        if (row) {
            const bar = row.querySelector(".option-progress-fill");
            const percentEl = row.querySelector(".option-percent");
            const votesEl = row.querySelector(".option-votes");
            
            bar.style.width = `${opt.percentage}%`;
            percentEl.textContent = `${opt.percentage}%`;
            votesEl.textContent = `${opt.votes} voto${opt.votes !== 1 ? 's' : ''}`;
            
            if (opt.isWinner && opt.votes > 0) {
                row.classList.add("winning-option");
            } else {
                row.classList.remove("winning-option");
            }
        }
    });
}

// Finalizar encuesta
function endPoll(pollData) {
    if (pollTimerInterval) clearInterval(pollTimerInterval);
    
    pollStatus.textContent = "ENCUESTA FINALIZADA";
    pollStatus.className = "badge ended-badge";
    pollTimer.textContent = "00:00";
    timerFill.style.width = "0%";
    
    // Mostrar el ganador definitivo
    updatePoll(pollData);
    
    // Ocultar perdedores y destacar ganador con transición
    setTimeout(() => {
        const calculated = calculatePercentages(pollData.options, pollData.total_votes);
        calculated.forEach(opt => {
            const row = document.getElementById(`opt-${opt.id}`);
            if (row) {
                if (opt.isWinner && opt.votes > 0) {
                    row.classList.add("winner-reveal");
                } else {
                    row.classList.add("fade-out-option");
                }
            }
        });
    }, 600);
    
    // Esperar 8 segundos antes de ocultar el widget
    setTimeout(() => {
        pollContainer.classList.add("hidden");
        activePoll = null;
    }, 8000);
}

// Calcular porcentajes e identificar ganadores
function calculatePercentages(options, totalVotes) {
    if (totalVotes === 0) {
        return options.map(opt => ({ ...opt, percentage: 0, isWinner: false }));
    }
    
    let maxVotes = -1;
    const items = options.map(opt => {
        const percentage = Math.round((opt.votes / totalVotes) * 100);
        if (opt.votes > maxVotes) {
            maxVotes = opt.votes;
        }
        return { ...opt, percentage };
    });
    
    // Marcar como ganador(es) a los que tengan el número máximo de votos (si hay más de 0 votos)
    return items.map(opt => ({
        ...opt,
        isWinner: opt.votes === maxVotes && maxVotes > 0
    }));
}

// Dibujar las opciones en la interfaz
function renderOptions(options) {
    optionsContainer.innerHTML = "";
    
    options.forEach(opt => {
        const row = document.createElement("div");
        row.className = "option-row";
        row.id = `opt-${opt.id}`;
        
        row.innerHTML = `
            <div class="option-progress-fill" style="width: 0%"></div>
            <span class="option-label">${opt.title}</span>
            <div class="option-values">
                <span class="option-percent">0%</span>
                <span class="option-votes">0 votos</span>
            </div>
            <div class="winner-badge">Ganador</div>
        `;
        
        optionsContainer.appendChild(row);
    });
}

// Actualizar barra de tiempo
function updateTimerUI() {
    const mins = Math.floor(durationLeft / 60);
    const secs = durationLeft % 60;
    pollTimer.textContent = `${mins.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}`;
    
    const fillPercent = (durationLeft / totalDuration) * 100;
    timerFill.style.width = `${fillPercent}%`;
}

// --- Simulación Demo para pruebas sin Twitch ---
function startDemoSimulation() {
    console.log("Iniciando modo demo de encuesta...");
    
    const demoPoll = {
        title: "¿Cuál será la próxima plataforma que probemos?",
        duration_seconds: 30,
        total_votes: 0,
        options: [
            { id: "opt_1", title: "YouTube Live", votes: 0 },
            { id: "opt_2", title: "Kick Gaming", votes: 0 },
            { id: "opt_3", title: "TikTok Studio", votes: 0 }
        ]
    };
    
    startPoll(demoPoll);
    
    // Simular votos entrantes cada 1-2 segundos
    let simInterval = setInterval(() => {
        if (!activePoll || durationLeft <= 0) {
            clearInterval(simInterval);
            
            // Simular fin de encuesta
            endPoll(activePoll);
            
            // Repetir el bucle de simulación tras un breve retraso
            setTimeout(() => {
                if (!socket || socket.readyState !== WebSocket.OPEN) {
                    startDemoSimulation();
                }
            }, 18000);
            return;
        }
        
        // Añadir votos aleatorios
        const randOptIndex = Math.floor(Math.random() * activePoll.options.length);
        activePoll.options[randOptIndex].votes += Math.floor(Math.random() * 3) + 1;
        activePoll.total_votes = activePoll.options.reduce((sum, o) => sum + o.votes, 0);
        
        updatePoll(activePoll);
    }, 1500);
}

// Iniciar
window.addEventListener("DOMContentLoaded", () => {
    connect();
    
    // Si abrimos la URL localmente (sin host de servidor de Rust), arranca demo automáticamente
    if (window.location.protocol === "file:") {
        startDemoSimulation();
    }
});
