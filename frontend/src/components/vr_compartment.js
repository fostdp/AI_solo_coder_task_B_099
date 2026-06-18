export class VRCompartment {
    constructor(options = {}) {
        this.apiBase = options.apiBase || '/api';
        this.currentShipId = options.currentShipId || '';
        this.currentShipConfig = options.currentShipConfig || null;
        this.doorStates = {};
        this.hapticEnabled = navigator.vibrate !== undefined;
        this.audioContext = null;
        this.onResult = options.onResult || null;
        this.onShipUpdate = options.onShipUpdate || null;
        this.onDoorChange = options.onDoorChange || null;
    }

    init() {
        this.updateDoorControls();

        const simulateBtn = document.getElementById('interactive-simulate-btn');
        if (simulateBtn) {
            simulateBtn.addEventListener('click', () => this.runInteractiveSimulation());
        }
    }

    setShip(shipId, shipConfig) {
        this.currentShipId = shipId;
        this.currentShipConfig = shipConfig;
        this.doorStates = {};
        this.updateDoorControls();
    }

    updateDoorControls() {
        const container = document.getElementById('door-controls');
        if (!container || !this.currentShipConfig) return;

        container.innerHTML = '';
        const numDoors = this.currentShipConfig.compartment_count - 1;

        for (let i = 0; i < numDoors; i++) {
            if (this.doorStates[i] === undefined) {
                this.doorStates[i] = false;
            }

            const div = document.createElement('div');
            div.className = 'door-control';
            div.id = `door-control-${i}`;
            if (!this.doorStates[i]) {
                div.classList.add('active');
            }

            const leftName = this.currentShipConfig.compartment_names[i] || `舱${i}`;
            const rightName = this.currentShipConfig.compartment_names[i + 1] || `舱${i + 1}`;

            div.innerHTML = `
                <span class="door-name">${i}号舱壁 (${leftName} ↔ ${rightName})</span>
                <span class="door-status ${this.doorStates[i] ? 'open' : 'closed'}" id="door-status-${i}">
                    ${this.doorStates[i] ? '开启' : '关闭'}
                </span>
                <label class="switch" id="switch-${i}">
                    <input type="checkbox" id="door-${i}" ${this.doorStates[i] ? 'checked' : ''} data-bulkhead="${i}" />
                    <span class="slider"></span>
                </label>
            `;
            container.appendChild(div);

            div.querySelector('input').addEventListener('change', (e) => this.handleDoorToggle(e));
        }
    }

    async handleDoorToggle(e) {
        const bulkheadId = parseInt(e.target.dataset.bulkhead);
        const isOpen = e.target.checked;
        const isClosing = !isOpen;

        this.doorStates[bulkheadId] = isOpen;

        const statusEl = document.getElementById(`door-status-${bulkheadId}`);
        if (statusEl) {
            statusEl.textContent = isOpen ? '开启' : '关闭';
            statusEl.className = `door-status ${isOpen ? 'open' : 'closed'}`;
        }

        const controlEl = document.getElementById(`door-control-${bulkheadId}`);
        const switchEl = document.getElementById(`switch-${bulkheadId}`);

        if (controlEl) {
            controlEl.classList.remove('active', 'door-locking', 'door-unlocking', 'force-feedback-pulse');
            void controlEl.offsetWidth;

            if (isClosing) {
                controlEl.classList.add('active', 'door-locking');
                if (switchEl) switchEl.classList.add('force-feedback-pulse');
                this.triggerHapticFeedback('lock');
                this.playDoorSound(true);
                this.showToast(
                    `🔒 ${bulkheadId}号水密舱门已关闭并锁闭<br><small>舱壁处于水密状态，可阻止进水蔓延</small>`,
                    'success',
                    2500
                );
            } else {
                controlEl.classList.remove('active');
                controlEl.classList.add('door-unlocking');
                this.triggerHapticFeedback('medium');
                this.playDoorSound(false);
                this.showToast(
                    `⚠️ ${bulkheadId}号水密舱门已开启<br><small>注意：舱门开启会导致进水通过此舱壁蔓延！</small>`,
                    'warning',
                    3000
                );
            }
        }

        if (this.onDoorChange) {
            this.onDoorChange(bulkheadId, isOpen);
        }

        try {
            await fetch(`${this.apiBase}/door`, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    ship_id: this.currentShipId,
                    bulkhead_id: bulkheadId,
                    open: isOpen
                })
            });
        } catch (error) {
            console.error('Door control failed:', error);
            this.showToast('舱门状态同步失败', 'warning');
        }
    }

    initAudioContext() {
        if (!this.audioContext) {
            try {
                this.audioContext = new (window.AudioContext || window.webkitAudioContext)();
            } catch (e) {
                console.log('Web Audio API not supported');
            }
        }
        return this.audioContext;
    }

    playDoorSound(isClosing) {
        const ctx = this.initAudioContext();
        if (!ctx) return;

        const now = ctx.currentTime;

        if (isClosing) {
            const osc1 = ctx.createOscillator();
            const gain1 = ctx.createGain();
            osc1.connect(gain1);
            gain1.connect(ctx.destination);
            osc1.type = 'sine';
            osc1.frequency.setValueAtTime(200, now);
            osc1.frequency.exponentialRampToValueAtTime(80, now + 0.1);
            gain1.gain.setValueAtTime(0.3, now);
            gain1.gain.exponentialRampToValueAtTime(0.01, now + 0.2);
            osc1.start(now);
            osc1.stop(now + 0.2);

            const osc2 = ctx.createOscillator();
            const gain2 = ctx.createGain();
            osc2.connect(gain2);
            gain2.connect(ctx.destination);
            osc2.type = 'square';
            osc2.frequency.setValueAtTime(400, now + 0.15);
            gain2.gain.setValueAtTime(0.15, now + 0.15);
            gain2.gain.exponentialRampToValueAtTime(0.01, now + 0.25);
            osc2.start(now + 0.15);
            osc2.stop(now + 0.25);
        } else {
            const osc = ctx.createOscillator();
            const gain = ctx.createGain();
            osc.connect(gain);
            gain.connect(ctx.destination);
            osc.type = 'sine';
            osc.frequency.setValueAtTime(100, now);
            osc.frequency.exponentialRampToValueAtTime(200, now + 0.15);
            gain.gain.setValueAtTime(0.25, now);
            gain.gain.exponentialRampToValueAtTime(0.01, now + 0.2);
            osc.start(now);
            osc.stop(now + 0.2);

            const noise = ctx.createBufferSource();
            const noiseBuffer = ctx.createBuffer(1, ctx.sampleRate * 0.1, ctx.sampleRate);
            const output = noiseBuffer.getChannelData(0);
            for (let i = 0; i < noiseBuffer.length; i++) {
                output[i] = Math.random() * 2 - 1;
            }
            noise.buffer = noiseBuffer;
            const noiseGain = ctx.createGain();
            noise.connect(noiseGain);
            noiseGain.connect(ctx.destination);
            noiseGain.gain.setValueAtTime(0.05, now);
            noiseGain.gain.exponentialRampToValueAtTime(0.01, now + 0.1);
            noise.start(now);
        }
    }

    triggerHapticFeedback(intensity = 'medium') {
        if (!this.hapticEnabled) return;

        try {
            switch (intensity) {
                case 'light':
                    navigator.vibrate(10);
                    break;
                case 'medium':
                    navigator.vibrate([30, 20, 30]);
                    break;
                case 'heavy':
                    navigator.vibrate([50, 30, 50, 30, 50]);
                    break;
                case 'lock':
                    navigator.vibrate([20, 50, 100]);
                    break;
            }
        } catch (e) {
            console.log('Haptic feedback not available');
        }
    }

    showToast(message, type = 'info', duration = 3000) {
        const container = document.getElementById('toast-container');
        if (!container) return;

        const toast = document.createElement('div');
        toast.className = `toast-notification ${type}`;
        toast.innerHTML = message;

        container.appendChild(toast);

        requestAnimationFrame(() => {
            toast.classList.add('show');
        });

        setTimeout(() => {
            toast.classList.remove('show');
            setTimeout(() => toast.remove(), 300);
        }, duration);
    }

    async runInteractiveSimulation() {
        const damageInput = document.getElementById('interactive-damage')?.value;
        const severity = parseFloat(document.getElementById('interactive-severity')?.value || '0.7');

        if (!damageInput) {
            this.showToast('请输入初始破损舱室', 'warning');
            return;
        }

        const compartments = damageInput.split(',').map(s => parseInt(s.trim())).filter(n => !isNaN(n));
        const doorStatesArray = Object.entries(this.doorStates).map(([id, isOpen]) => ({
            bulkhead_id: parseInt(id),
            is_open: isOpen,
            last_changed: new Date().toISOString()
        }));

        const payload = {
            ship_id: this.currentShipId,
            flooded_compartments: compartments,
            damage_severity: severity,
            door_states: doorStatesArray
        };

        const btn = document.getElementById('interactive-simulate-btn');
        try {
            btn.textContent = '仿真中...';
            btn.disabled = true;

            const response = await fetch(`${this.apiBase}/simulate/interactive`, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(payload)
            });

            if (response.ok) {
                const result = await response.json();
                this.renderInteractiveResult(result, doorStatesArray);
                if (this.onShipUpdate) {
                    this.onShipUpdate(result);
                }
                if (this.onResult) {
                    this.onResult(result);
                }
            }
        } catch (error) {
            console.error('Interactive simulation failed:', error);
            this.showToast('仿真失败，请重试', 'warning');
        } finally {
            btn.textContent = '基于当前舱门状态仿真';
            btn.disabled = false;
        }
    }

    renderInteractiveResult(result, doorStates) {
        const resultsDiv = document.getElementById('interactive-result');
        const conclusion = document.getElementById('interactive-conclusion');

        if (!resultsDiv) return;

        const openDoors = doorStates.filter(d => d.is_open).length;
        const totalDoors = doorStates.length;
        const floodedCount = result.flooded_compartments?.length || 0;

        let message = '';
        if (result.is_safe) {
            message = `✅ <strong>船舶保持安全！</strong><br><br>`;
        } else {
            message = `❌ <strong>船舶处于危险状态！</strong><br><br>`;
        }

        message += `
            <strong>操作分析：</strong>您开启了 ${openDoors}/${totalDoors} 道舱门，初始破损 ${floodedCount} 个舱室。<br><br>
            <strong>仿真结果：</strong><br>
            • 最终进水舱室: ${floodedCount} 个<br>
            • 最终吃水: ${result.final_draft?.toFixed(2) || '0.00'} m<br>
            • 初稳心高 GM: ${result.metacentric_height?.toFixed(3) || '0.000'} m<br>
            • 横倾角: ${result.heel_angle?.toFixed(2) || '0.00'}°
        `;

        if (conclusion) {
            conclusion.innerHTML = message;
        }

        resultsDiv.style.display = 'block';
    }

    getDoorStates() {
        return { ...this.doorStates };
    }

    setDoorState(bulkheadId, isOpen) {
        this.doorStates[bulkheadId] = isOpen;
        this.updateDoorControls();
    }

    closeAllDoors() {
        Object.keys(this.doorStates).forEach(id => {
            this.doorStates[id] = false;
        });
        this.updateDoorControls();
    }

    openAllDoors() {
        Object.keys(this.doorStates).forEach(id => {
            this.doorStates[id] = true;
        });
        this.updateDoorControls();
    }
}

