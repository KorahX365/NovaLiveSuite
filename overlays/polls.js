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
let isPrediction = false;
let durationLeft = 0;
let totalDuration = 0;

// Configuración de WebSocket
function connect() {
    const host = window.location.host || "localhost:777";
    const wsUrl = `ws://${host}/obs/polls/ws`;
    
    socket = new WebSocket(wsUrl);
    
    socket.onopen = () => {
        console.log("Overlay connected to WS server");
        if (reconnectTimer) {
            clearTimeout(reconnectTimer);
            reconnectTimer = null;
        }
    };
    
    socket.onmessage = (event) => {
        try {
            const data = JSON.parse(event.data);
            
            // Handlers para Encuestas de Twitch
            if (data.event === "poll_begin") {
                isPrediction = false;
                startPoll(data.data);
            } else if (data.event === "poll_progress") {
                isPrediction = false;
                updatePoll(data.data);
            } else if (data.event === "poll_end") {
                isPrediction = false;
                endPoll(data.data);
            } 
            // Handlers para Predicciones de Twitch
            else if (data.event === "prediction_begin") {
                isPrediction = true;
                startPrediction(data.data);
            } else if (data.event === "prediction_progress") {
                isPrediction = true;
                updatePrediction(data.data);
            } else if (data.event === "prediction_lock") {
                isPrediction = true;
                lockPrediction(data.data);
            } else if (data.event === "prediction_end") {
                isPrediction = true;
                endPrediction(data.data);
            } else if (data.event === "poll_reset") {
                resetPollUI();
            }
        } catch (e) {
            console.error("Error parsing WebSocket message:", e);
        }
    };
    
    socket.onclose = () => {
        console.log("WebSocket connection lost. Reconnecting in 3s...");
        reconnectTimer = setTimeout(connect, 3000);
        
        // Simulación demo en local
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

// Format numbers (e.g. 1500 -> 1.5k)
function formatNumber(num) {
    if (num >= 1000000) {
        return (num / 1000000).toFixed(1).replace(/\.0$/, '') + 'M';
    }
    if (num >= 1000) {
        return (num / 1000).toFixed(1).replace(/\.0$/, '') + 'k';
    }
    return num.toString();
}

// --- LÓGICA DE ENCUESTAS (POLLS) ---
function startPoll(pollData) {
    activePoll = pollData;
    isPrediction = false;
    
    pollTitle.textContent = pollData.title;
    pollStatus.textContent = "ENCUESTA ACTIVA";
    pollStatus.className = "badge active-badge";
    pollTotalVotes.textContent = `${formatNumber(pollData.total_votes || 0)} votos`;
    
    totalDuration = pollData.duration_seconds || 60;
    durationLeft = totalDuration;
    updateTimerUI();
    
    if (pollTimerInterval) clearInterval(pollTimerInterval);
    pollTimerInterval = setInterval(() => {
        durationLeft--;
        if (durationLeft <= 0) {
            durationLeft = 0;
            clearInterval(pollTimerInterval);
        }
        updateTimerUI();
    }, 1000);
    
    renderOptions(pollData.options);
    pollContainer.classList.remove("hidden");
}

function updatePoll(pollData) {
    if (!activePoll) return;
    activePoll.total_votes = pollData.total_votes;
    pollTotalVotes.textContent = `${formatNumber(pollData.total_votes)} votos`;
    
    const options = calculatePercentages(pollData.options, pollData.total_votes);
    
    options.forEach(opt => {
        const row = document.getElementById(`opt-${opt.id}`);
        if (row) {
            const bar = row.querySelector(".option-progress-fill");
            const percentEl = row.querySelector(".option-percent");
            const votesEl = row.querySelector(".option-votes");
            
            bar.style.width = `${opt.percentage}%`;
            percentEl.textContent = `${opt.percentage}%`;
            votesEl.textContent = `${formatNumber(opt.votes)} voto${opt.votes !== 1 ? 's' : ''}`;
            
            if (opt.isWinner && opt.votes > 0) {
                row.classList.add("winning-option");
            } else {
                row.classList.remove("winning-option");
            }
        }
    });
}

function endPoll(pollData) {
    if (pollTimerInterval) clearInterval(pollTimerInterval);
    
    pollStatus.textContent = "ENCUESTA FINALIZADA";
    pollStatus.className = "badge ended-badge";
    pollTimer.textContent = "00:00";
    timerFill.style.width = "0%";
    
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
    
    setTimeout(() => {
        pollContainer.classList.add("hidden");
        activePoll = null;
    }, 8000);
}

// --- LÓGICA DE PREDICCIONES (PREDICTIONS) ---
function startPrediction(predData) {
    activePoll = predData;
    isPrediction = true;
    
    pollTitle.textContent = predData.title;
    pollStatus.textContent = "PREDICCIÓN ACTIVA";
    pollStatus.className = "badge active-badge";
    
    const totalPoints = predData.outcomes.reduce((sum, o) => sum + (o.channel_points || 0), 0);
    pollTotalVotes.textContent = `${formatNumber(totalPoints)} pts`;
    
    totalDuration = predData.lock_duration || 60;
    durationLeft = totalDuration;
    updateTimerUI();
    
    if (pollTimerInterval) clearInterval(pollTimerInterval);
    pollTimerInterval = setInterval(() => {
        durationLeft--;
        if (durationLeft <= 0) {
            durationLeft = 0;
            clearInterval(pollTimerInterval);
        }
        updateTimerUI();
    }, 1000);
    
    renderPredictionOutcomes(predData.outcomes, totalPoints);
    pollContainer.classList.remove("hidden");
}

function updatePrediction(predData) {
    if (!activePoll) return;
    
    const totalPoints = predData.outcomes.reduce((sum, o) => sum + (o.channel_points || 0), 0);
    pollTotalVotes.textContent = `${formatNumber(totalPoints)} pts`;
    
    // Mapear ratios y porcentajes
    const outcomes = calculatePredictionRatios(predData.outcomes, totalPoints);
    
    outcomes.forEach(out => {
        const row = document.getElementById(`opt-${out.id}`);
        if (row) {
            const bar = row.querySelector(".option-progress-fill");
            const percentEl = row.querySelector(".option-percent");
            const votesEl = row.querySelector(".option-votes");
            
            bar.style.width = `${out.percentage}%`;
            percentEl.textContent = `${out.percentage}%`;
            votesEl.textContent = `${formatNumber(out.channel_points)} pts (1:${out.ratio})`;
            
            if (out.isWinner && out.channel_points > 0) {
                row.classList.add("winning-option");
            } else {
                row.classList.remove("winning-option");
            }
        }
    });
}

function lockPrediction(predData) {
    if (pollTimerInterval) clearInterval(pollTimerInterval);
    pollStatus.textContent = "PREDICCIÓN CERRADA";
    pollStatus.className = "badge ended-badge";
    pollTimer.textContent = "00:00";
    timerFill.style.width = "0%";
    
    updatePrediction(predData);
}

function endPrediction(predData) {
    if (pollTimerInterval) clearInterval(pollTimerInterval);
    
    pollStatus.textContent = "PREDICCIÓN FINALIZADA";
    pollStatus.className = "badge ended-badge";
    pollTimer.textContent = "00:00";
    timerFill.style.width = "0%";
    
    const totalPoints = predData.outcomes.reduce((sum, o) => sum + (o.channel_points || 0), 0);
    pollTotalVotes.textContent = `${formatNumber(totalPoints)} pts`;
    
    // Identificar el ID del outcome ganador
    const winningOutcomeId = predData.winning_outcome_id;
    const outcomes = calculatePredictionRatios(predData.outcomes, totalPoints);
    
    // Actualizar barras
    outcomes.forEach(out => {
        const row = document.getElementById(`opt-${out.id}`);
        if (row) {
            const bar = row.querySelector(".option-progress-fill");
            const percentEl = row.querySelector(".option-percent");
            const votesEl = row.querySelector(".option-votes");
            
            bar.style.width = `${out.percentage}%`;
            percentEl.textContent = `${out.percentage}%`;
            votesEl.textContent = `${formatNumber(out.channel_points)} pts (1:${out.ratio})`;
            
            if (out.id === winningOutcomeId) {
                row.classList.add("winning-option");
            } else {
                row.classList.remove("winning-option");
            }
        }
    });
    
    // Ocultar perdedores y destacar ganador definitivo
    setTimeout(() => {
        outcomes.forEach(out => {
            const row = document.getElementById(`opt-${out.id}`);
            if (row) {
                if (out.id === winningOutcomeId) {
                    row.classList.add("winner-reveal");
                } else {
                    row.classList.add("fade-out-option");
                }
            }
        });
    }, 600);
    
    setTimeout(() => {
        pollContainer.classList.add("hidden");
        activePoll = null;
    }, 10000);
}

// --- MÉTODOS DE SOPORTE ---

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
    
    return items.map(opt => ({
        ...opt,
        isWinner: opt.votes === maxVotes && maxVotes > 0
    }));
}

function calculatePredictionRatios(outcomes, totalPoints) {
    if (totalPoints === 0) {
        return outcomes.map(o => ({ ...o, percentage: 0, ratio: "1.00", isWinner: false }));
    }
    
    // Encontrar bando con más puntos temporalmente
    let maxPoints = -1;
    outcomes.forEach(o => {
        if ((o.channel_points || 0) > maxPoints) {
            maxPoints = o.channel_points || 0;
        }
    });
    
    return outcomes.map(o => {
        const pts = o.channel_points || 0;
        const percentage = Math.round((pts / totalPoints) * 100);
        
        // Calcular el retorno de Twitch: Total / Puntos del Bando
        let ratio = "1.00";
        if (pts > 0) {
            ratio = (totalPoints / pts).toFixed(2);
        }
        
        return {
            ...o,
            percentage,
            ratio,
            isWinner: pts === maxPoints && maxPoints > 0
        };
    });
}

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

function renderPredictionOutcomes(outcomes, totalPoints) {
    optionsContainer.innerHTML = "";
    
    const colors = ["rgba(59, 130, 246, 0.4)", "rgba(236, 72, 153, 0.4)"]; // Azul vs Rosa
    
    outcomes.forEach((out, idx) => {
        const row = document.createElement("div");
        row.className = "option-row";
        row.id = `opt-${out.id}`;
        
        // Personalizar color de borde segun el bando
        const borderStyle = idx === 0 ? "rgba(59, 130, 246, 0.25)" : "rgba(236, 72, 153, 0.25)";
        row.style.borderColor = borderStyle;
        
        row.innerHTML = `
            <div class="option-progress-fill" style="width: 0%"></div>
            <span class="option-label">${out.title}</span>
            <div class="option-values">
                <span class="option-percent">0%</span>
                <span class="option-votes">0 pts (1:1.00)</span>
            </div>
            <div class="winner-badge" style="background: ${idx === 0 ? '#3b82f6' : '#ec4899'}">Ganador</div>
        `;
        optionsContainer.appendChild(row);
    });
}

function updateTimerUI() {
    const mins = Math.floor(durationLeft / 60);
    const secs = durationLeft % 60;
    pollTimer.textContent = `${mins.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}`;
    
    const fillPercent = (durationLeft / totalDuration) * 100;
    timerFill.style.width = `${fillPercent}%`;
}

// --- Simulación Demo para pruebas sin Twitch ---
function startDemoSimulation() {
    console.log("Iniciando simulación demo...");
    
    // Alternar entre encuesta y predicción en la demo cada ciclo
    const runAsPrediction = Math.random() > 0.5;
    
    if (runAsPrediction) {
        // Demostración de Predicción
        const demoPred = {
            title: "¿Logrará ganar la partida en menos de 10 min?",
            lock_duration: 30,
            outcomes: [
                { id: "outcome_1", title: "SÍ, ES UN PRO 🏆", channel_points: 0 },
                { id: "outcome_2", title: "NO, F EN EL CHAT 💀", channel_points: 0 }
            ]
        };
        
        startPrediction(demoPred);
        
        let simInterval = setInterval(() => {
            if (!activePoll || durationLeft <= 0) {
                clearInterval(simInterval);
                
                // Bloquear predicción
                lockPrediction(activePoll);
                
                // Esperar 2s y finalizar con ganador
                setTimeout(() => {
                    if (activePoll) {
                        activePoll.winning_outcome_id = Math.random() > 0.5 ? "outcome_1" : "outcome_2";
                        endPrediction(activePoll);
                    }
                }, 2000);
                
                // Reiniciar loop demo
                setTimeout(() => {
                    if (!socket || socket.readyState !== WebSocket.OPEN) {
                        startDemoSimulation();
                    }
                }, 22000);
                return;
            }
            
            // Añadir puntos de canal aleatorios a los bandos
            const randIdx = Math.floor(Math.random() * activePoll.outcomes.length);
            activePoll.outcomes[randIdx].channel_points += Math.floor(Math.random() * 800) + 150;
            updatePrediction(activePoll);
        }, 1500);
        
    } else {
        // Demostración de Encuesta
        const demoPoll = {
            title: "¿Qué juego deberíamos probar hoy?",
            duration_seconds: 30,
            total_votes: 0,
            options: [
                { id: "opt_1", title: "Minecraft Hardcore", votes: 0 },
                { id: "opt_2", title: "Valorant Competitivo", votes: 0 },
                { id: "opt_3", title: "Elden Ring DLC", votes: 0 }
            ]
        };
        
        startPoll(demoPoll);
        
        let simInterval = setInterval(() => {
            if (!activePoll || durationLeft <= 0) {
                clearInterval(simInterval);
                endPoll(activePoll);
                
                setTimeout(() => {
                    if (!socket || socket.readyState !== WebSocket.OPEN) {
                        startDemoSimulation();
                    }
                }, 20000);
                return;
            }
            
            const randIdx = Math.floor(Math.random() * activePoll.options.length);
            activePoll.options[randIdx].votes += Math.floor(Math.random() * 3) + 1;
            activePoll.total_votes = activePoll.options.reduce((sum, o) => sum + o.votes, 0);
            updatePoll(activePoll);
        }, 1500);
    }
}

function resetPollUI() {
    if (pollTimerInterval) {
        clearInterval(pollTimerInterval);
        pollTimerInterval = null;
    }
    pollContainer.classList.add("hidden");
    activePoll = null;
    optionsContainer.innerHTML = "";
    console.log("Poll/Prediction overlay reset successfully.");
}

// Iniciar
window.addEventListener("DOMContentLoaded", () => {
    connect();
    if (window.location.protocol === "file:") {
        startDemoSimulation();
    }
});
