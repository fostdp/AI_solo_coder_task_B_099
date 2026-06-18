export class ExtremeFloodingSimulator {
    constructor(options = {}) {
        this.apiBase = options.apiBase || '/api';
        this.currentShipId = options.currentShipId || '';
        this.currentShipConfig = options.currentShipConfig || null;
        this.attackPoints = [];
        this.onResult = options.onResult || null;
        this.onShipUpdate = options.onShipUpdate || null;
    }

    init() {
        this.attackPoints = [];
        this.initAttackPoints();

        const addBtn = document.getElementById('add-attack-btn');
        if (addBtn) {
            addBtn.addEventListener('click', () => this.addAttackPoint());
        }

        const attackBtn = document.getElementById('pirate-attack-btn');
        if (attackBtn) {
            attackBtn.addEventListener('click', () => this.runSimulation());
        }
    }

    setShip(shipId, shipConfig) {
        this.currentShipId = shipId;
        this.currentShipConfig = shipConfig;
        this.initAttackPoints();
    }

    initAttackPoints() {
        this.attackPoints = [];
        const container = document.getElementById('attack-points');
        if (!container) return;

        container.innerHTML = '';

        if (this.currentShipConfig) {
            const numCompartments = this.currentShipConfig.compartment_count;
            this.addAttackPoint(Math.floor(numCompartments / 3), 0.7, 0);
            this.addAttackPoint(Math.floor(numCompartments / 2), 0.8, 60);
        }
    }

    addAttackPoint(compartmentId = 3, severity = 0.7, delay = 0) {
        const container = document.getElementById('attack-points');
        if (!container) return;

        const index = this.attackPoints.length;
        const maxCompartment = this.currentShipConfig
            ? this.currentShipConfig.compartment_count - 1
            : 12;

        const div = document.createElement('div');
        div.className = 'attack-point-row';
        div.innerHTML = `
            <input type="number" class="attack-compartment" value="${compartmentId}" min="0" max="${maxCompartment}" placeholder="舱室" />
            <input type="number" class="attack-severity" value="${severity}" min="0" max="1" step="0.1" placeholder="严重度" />
            <input type="number" class="attack-delay" value="${delay}" min="0" placeholder="延迟秒" />
            <button class="remove-attack-btn" data-index="${index}">×</button>
        `;
        container.appendChild(div);

        this.attackPoints.push({
            compartment_id: compartmentId,
            severity: severity,
            delay_seconds: delay
        });

        div.querySelector('.remove-attack-btn').addEventListener('click', () => {
            this.removeAttackPoint(index);
        });

        div.querySelector('.attack-compartment').addEventListener('change', (e) => {
            this.attackPoints[index].compartment_id = parseInt(e.target.value) || 0;
        });

        div.querySelector('.attack-severity').addEventListener('change', (e) => {
            this.attackPoints[index].severity = parseFloat(e.target.value) || 0;
        });

        div.querySelector('.attack-delay').addEventListener('change', (e) => {
            this.attackPoints[index].delay_seconds = parseFloat(e.target.value) || 0;
        });
    }

    removeAttackPoint(index) {
        this.attackPoints.splice(index, 1);
        this.updateAttackPointIndices();
    }

    updateAttackPointIndices() {
        const container = document.getElementById('attack-points');
        if (!container) return;

        const rows = container.querySelectorAll('.attack-point-row');
        rows.forEach((row, i) => {
            const removeBtn = row.querySelector('.remove-attack-btn');
            if (removeBtn) {
                removeBtn.dataset.index = i;
            }
        });
    }

    async runSimulation() {
        if (this.attackPoints.length === 0) {
            alert('请至少添加一个攻击点');
            return;
        }

        const duration = parseInt(document.getElementById('attack-duration')?.value || '600');
        const payload = {
            ship_id: this.currentShipId,
            attack_points: this.attackPoints,
            simulation_duration_seconds: duration
        };

        const btn = document.getElementById('pirate-attack-btn');
        try {
            btn.textContent = '仿真中...';
            btn.disabled = true;

            const response = await fetch(`${this.apiBase}/simulate/pirate`, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(payload)
            });

            if (response.ok) {
                const result = await response.json();
                this.renderResults(result);
                if (this.onResult) {
                    this.onResult(result);
                }
                if (this.onShipUpdate && result.final_state) {
                    this.onShipUpdate(result.final_state);
                }
            }
        } catch (error) {
            console.error('Pirate attack simulation failed:', error);
        } finally {
            btn.textContent = '开始海盗攻击仿真';
            btn.disabled = false;
        }
    }

    renderResults(result) {
        const resultsDiv = document.getElementById('pirate-results');
        const indicator = document.getElementById('survival-indicator');
        const percent = document.getElementById('survival-percent');
        const timeline = document.getElementById('attack-timeline');
        const critical = document.getElementById('critical-moments');

        if (!resultsDiv) return;

        const survivalPct = (result.survival_probability * 100).toFixed(1);
        if (percent) {
            percent.textContent = `${survivalPct}%`;
        }

        if (indicator) {
            if (result.survival_probability < 0.3) {
                indicator.classList.add('danger');
            } else {
                indicator.classList.remove('danger');
            }
        }

        if (timeline) {
            timeline.innerHTML = '';
            (result.timeline_events || []).forEach(event => {
                const div = document.createElement('div');
                div.className = `timeline-event ${event.event_type === 'UNSAFE' ? 'critical' : event.event_type === 'SURVIVED' ? 'safe' : ''}`;
                div.innerHTML = `
                    <div class="time">${event.time_seconds?.toFixed(0) || 0}s</div>
                    <div class="desc">${event.description || ''}</div>
                    <div style="font-size: 10px; color: #888; margin-top: 2px;">
                        进水舱: ${(event.affected_compartments || []).join(', ')} | GM: ${event.gm_value?.toFixed(3) || 0}m | ${event.is_safe ? '✅安全' : '❌危险'}
                    </div>
                `;
                timeline.appendChild(div);
            });
        }

        if (critical) {
            critical.innerHTML = '';
            if ((result.critical_moments || []).length > 0) {
                result.critical_moments.forEach(moment => {
                    const div = document.createElement('div');
                    div.style.cssText = 'padding: 8px; background: rgba(255, 107, 107, 0.1); border-left: 3px solid #ff6b6b; border-radius: 4px; margin-bottom: 5px; font-size: 11px;';
                    div.innerHTML = `
                        <strong>${moment.time_seconds?.toFixed(0) || 0}s</strong> - ${moment.description || ''}
                        <div style="color: #ff6b6b; font-size: 10px; margin-top: 2px;">GM = ${moment.gm_value?.toFixed(3) || 0}m</div>
                    `;
                    critical.appendChild(div);
                });
            } else {
                critical.innerHTML = '<div style="color: #2ed573; font-size: 11px; text-align: center; padding: 10px;">✅ 无危险时刻，船舶状态良好</div>';
            }
        }

        resultsDiv.style.display = 'block';
    }
}

