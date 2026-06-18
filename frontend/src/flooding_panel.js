import { Chart, registerables } from 'chart.js';
import { JunkShip3D } from './junk_ship_3d.js';
import { DesignComparator } from './components/design_comparator.js';
import { EraComparator } from './components/era_comparator.js';
import { ExtremeFloodingSimulator } from './components/extreme_simulator.js';
import { VRCompartment } from './components/vr_compartment.js';

Chart.register(...registerables);

const API_BASE = 'http://localhost:8080';
const WS_BASE = 'ws://localhost:8080/ws';

const COMPARTMENT_NAMES = [
    "艏尖舱", "前货舱1", "前货舱2", "中货舱1", "中货舱2",
    "中货舱3", "中货舱4", "后货舱1", "后货舱2", "机舱",
    "艉尖舱", "淡水舱1", "淡水舱2"
];

class FloodingPanel {
    constructor() {
        this.ship3D = null;
        this.stabilityChart = null;
        this.draftChart = null;
        this.waterChart = null;
        this.ws = null;
        this.currentShipId = 'quanzhou_song_001';
        this.currentShipConfig = null;
        this.draftHistory = [];
        this.timeLabels = [];
        this.waterLevelHistory = {};
        this.alarms = [];
        this.latestSensorData = null;
        this.latestSimulationResult = null;

        this.designComparator = null;
        this.eraComparator = null;
        this.extremeSimulator = null;
        this.vrCompartment = null;

        this.init();
    }

    init() {
        this.ship3D = new JunkShip3D('canvas-container');
        window.ship3D = this.ship3D;
        this.initCharts();
        this.initWebSocket();
        this.initUI();
        this.loadShipConfig();
        this.initCompartmentList();
        this.initTabs();
        this.initShipSelector();

        this.initComponents();
    }

    initComponents() {
        this.designComparator = new DesignComparator({
            apiBase: API_BASE + '/api',
            onResult: (result) => {
                console.log('Design comparison result:', result);
            }
        });
        this.designComparator.init();

        this.eraComparator = new EraComparator({
            apiBase: API_BASE + '/api',
            onResult: (result) => {
                console.log('Era comparison result:', result);
            }
        });
        this.eraComparator.init();

        this.extremeSimulator = new ExtremeFloodingSimulator({
            apiBase: API_BASE + '/api',
            currentShipId: this.currentShipId,
            onResult: (result) => {
                if (result.final_state) {
                    this.handleSimulationResult(result.final_state);
                }
                if (window.ship3D) {
                    window.ship3D.showPirateAttackEffect();
                }
            },
            onShipUpdate: (state) => {
                this.handleSimulationResult(state);
            }
        });

        this.vrCompartment = new VRCompartment({
            apiBase: API_BASE + '/api',
            currentShipId: this.currentShipId,
            onShipUpdate: (result) => {
                this.handleSimulationResult(result);
            },
            onDoorChange: (bulkheadId, isOpen) => {
                console.log(`Door ${bulkheadId} changed to ${isOpen}`);
            }
        });
    }

    initCharts() {
        const chartOptions = {
            responsive: true,
            maintainAspectRatio: false,
            plugins: {
                legend: {
                    labels: {
                        color: '#e8e8e8',
                        font: { size: 11 }
                    }
                }
            },
            scales: {
                x: {
                    ticks: { color: '#888', font: { size: 10 } },
                    grid: { color: 'rgba(255,255,255,0.1)' }
                },
                y: {
                    ticks: { color: '#888', font: { size: 10 } },
                    grid: { color: 'rgba(255,255,255,0.1)' }
                }
            }
        };

        this.stabilityChart = new Chart(
            document.getElementById('stability-chart'),
            {
                type: 'line',
                data: {
                    labels: Array.from({ length: 91 }, (_, i) => i),
                    datasets: [{
                        label: '复原力臂 GZ (m)',
                        data: [],
                        borderColor: '#4facfe',
                        backgroundColor: 'rgba(79, 172, 254, 0.2)',
                        fill: true,
                        tension: 0.3,
                        pointRadius: 0
                    }]
                },
                options: {
                    ...chartOptions,
                    scales: {
                        ...chartOptions.scales,
                        x: { ...chartOptions.scales.x, title: { display: true, text: '横倾角 (°)', color: '#ffd700' } },
                        y: { ...chartOptions.scales.y, title: { display: true, text: '复原力臂 (m)', color: '#ffd700' } }
                    }
                }
            }
        );

        this.draftChart = new Chart(
            document.getElementById('draft-chart'),
            {
                type: 'line',
                data: {
                    labels: [],
                    datasets: [{
                        label: '吃水深度 (m)',
                        data: [],
                        borderColor: '#ff6b6b',
                        backgroundColor: 'rgba(255, 107, 107, 0.2)',
                        fill: true,
                        tension: 0.4
                    }, {
                        label: '横倾角 (°)',
                        data: [],
                        borderColor: '#ffd700',
                        backgroundColor: 'rgba(255, 215, 0, 0.1)',
                        fill: false,
                        tension: 0.4,
                        yAxisID: 'y1'
                    }]
                },
                options: {
                    ...chartOptions,
                    scales: {
                        ...chartOptions.scales,
                        x: { ...chartOptions.scales.x, title: { display: true, text: '时间', color: '#ffd700' } },
                        y: { ...chartOptions.scales.y, title: { display: true, text: '吃水 (m)', color: '#ffd700' } },
                        y1: {
                            position: 'right',
                            title: { display: true, text: '横倾角 (°)', color: '#ffd700' },
                            ticks: { color: '#888', font: { size: 10 } },
                            grid: { drawOnChartArea: false }
                        }
                    }
                }
            }
        );

        this.waterChart = new Chart(
            document.getElementById('water-chart'),
            {
                type: 'bar',
                data: {
                    labels: COMPARTMENT_NAMES,
                    datasets: [{
                        label: '水位 (m)',
                        data: new Array(COMPARTMENT_NAMES.length).fill(0),
                        backgroundColor: COMPARTMENT_NAMES.map(() => 'rgba(79, 172, 254, 0.6)'),
                        borderColor: '#4facfe',
                        borderWidth: 1
                    }]
                },
                options: {
                    ...chartOptions,
                    scales: {
                        ...chartOptions.scales,
                        x: {
                            ...chartOptions.scales.x,
                            ticks: { ...chartOptions.scales.x.ticks, maxRotation: 45, minRotation: 45 }
                        },
                        y: {
                            ...chartOptions.scales.y,
                            title: { display: true, text: '水位 (m)', color: '#ffd700' },
                            beginAtZero: true
                        }
                    }
                }
            }
        );

        this.generateInitialStabilityCurve();
    }

    generateInitialStabilityCurve() {
        const gm = 0.5;
        const curveData = [];
        for (let angle = 0; angle <= 90; angle++) {
            const rad = angle * Math.PI / 180;
            let gz = gm * Math.sin(rad);
            if (angle > 30) {
                gz *= Math.max(0.3, 1 - (angle - 30) / 15);
            }
            curveData.push(Math.max(0, gz));
        }
        this.stabilityChart.data.datasets[0].data = curveData;
        this.stabilityChart.update();
    }

    initWebSocket() {
        const wsUrl = `${WS_BASE}?ship_id=${this.currentShipId}`;
        this.ws = new WebSocket(wsUrl);

        this.ws.onopen = () => {
            console.log('WebSocket connected');
            this.updateConnectionStatus(true);
            this.ws.send(JSON.stringify({
                message_type: 'subscribe',
                data: this.currentShipId
            }));
        };

        this.ws.onmessage = (event) => {
            try {
                const msg = JSON.parse(event.data);
                this.handleWebSocketMessage(msg);
            } catch (e) {
                console.error('WebSocket message parse error:', e);
            }
        };

        this.ws.onclose = () => {
            console.log('WebSocket disconnected');
            this.updateConnectionStatus(false);
            setTimeout(() => this.initWebSocket(), 3000);
        };

        this.ws.onerror = (error) => {
            console.error('WebSocket error:', error);
            this.updateConnectionStatus(false);
        };
    }

    handleWebSocketMessage(msg) {
        switch (msg.message_type) {
            case 'sensor_data':
                this.handleSensorData(msg.data);
                break;
            case 'simulation_result':
                this.handleSimulationResult(msg.data);
                break;
            case 'alarm':
                this.handleAlarm(msg.data);
                break;
        }
    }

    handleSensorData(data) {
        if (!Array.isArray(data) || data.length === 0) return;

        this.latestSensorData = data;

        const first = data[0];
        this.updateStatusDisplay(first);
        this.updateCompartmentStates(data);
        this.updateWaterChart(data);
        this.updateDraftHistory(first);

        data.forEach(d => {
            this.ship3D.updateWaterLevel(d.compartment_id, d.water_level, d.max_water_level);
            this.ship3D.setFlooded(d.compartment_id, d.is_flooded);
        });

        this.ship3D.updateShipPose(first.draft, first.heel_angle, first.trim_angle);
    }

    handleSimulationResult(result) {
        this.latestSimulationResult = result;

        this.updateStatusDisplay({
            draft: result.final_draft,
            heel_angle: result.final_heel_angle,
            trim_angle: result.final_trim_angle,
            metacentric_height: result.metacentric_height,
            righting_arm: result.righting_arm_max
        });

        if (result.stability_curve) {
            this.updateStabilityChart(result.stability_curve);
        }

        document.getElementById('buoyancy-value').textContent = `${result.reserve_buoyancy?.toFixed(1) || '0.0'}%`;
        const buoyancyEl = document.getElementById('buoyancy-value');
        buoyancyEl.className = 'value ' + (result.reserve_buoyancy > 20 ? 'success' : result.reserve_buoyancy > 10 ? '' : 'danger');

        if (result.is_safe) {
            document.getElementById('ship-name').className = 'status-item connected';
        } else {
            document.getElementById('ship-name').className = 'status-item warning';
        }

        if (result.flooded_compartments) {
            const targetLevels = result.flooded_compartments.map(() => 2.5);
            this.ship3D.startFloodingAnimation(result.flooded_compartments, targetLevels, 5000);
        }
    }

    handleAlarm(alarm) {
        this.alarms.unshift(alarm);
        if (this.alarms.length > 20) this.alarms.pop();

        this.updateAlarmDisplay();
        this.playAlarmSound();
    }

    updateAlarmDisplay() {
        const container = document.getElementById('alarms-panel');

        if (this.alarms.length === 0) {
            container.innerHTML = '<div style="color: #888; font-size: 12px; text-align: center; padding: 20px;">暂无告警</div>';
            return;
        }

        container.innerHTML = this.alarms.map(alarm => {
            const levelClass = alarm.alarm_level === 'Critical' ? 'critical' : 'warning';
            const time = new Date(alarm.timestamp).toLocaleTimeString();
            return `
                <div class="alarm-item ${levelClass}">
                    <div style="font-weight: bold; margin-bottom: 3px;">
                        [${time}] ${this.getAlarmTypeName(alarm.alarm_type)}
                    </div>
                    <div>${alarm.description}</div>
                </div>
            `;
        }).join('');
    }

    getAlarmTypeName(type) {
        const names = {
            'StabilityLoss': '稳性丧失',
            'FloodingSpread': '进水蔓延',
            'DraftExceeded': '吃水超限',
            'HeelExcessive': '横倾过大'
        };
        return names[type] || type;
    }

    playAlarmSound() {
        const audioContext = new (window.AudioContext || window.webkitAudioContext)();
        const oscillator = audioContext.createOscillator();
        const gainNode = audioContext.createGain();

        oscillator.connect(gainNode);
        gainNode.connect(audioContext.destination);

        oscillator.frequency.value = 800;
        oscillator.type = 'square';
        gainNode.gain.setValueAtTime(0.1, audioContext.currentTime);
        gainNode.gain.exponentialRampToValueAtTime(0.01, audioContext.currentTime + 0.3);

        oscillator.start(audioContext.currentTime);
        oscillator.stop(audioContext.currentTime + 0.3);
    }

    initUI() {
        document.getElementById('simulate-btn').addEventListener('click', () => this.runSimulation());
        document.getElementById('reset-btn').addEventListener('click', () => this.resetShip());
        document.getElementById('animate-btn').addEventListener('click', () => this.playDemoAnimation());
        document.getElementById('optimize-btn').addEventListener('click', () => this.runOptimization());

        document.getElementById('view-side').addEventListener('click', () => this.ship3D.setView('side'));
        document.getElementById('view-top').addEventListener('click', () => this.ship3D.setView('top'));
        document.getElementById('view-3d').addEventListener('click', () => this.ship3D.setView('3d'));
    }

    async loadShipConfig() {
        try {
            const response = await fetch(`/api/ships/${this.currentShipId}`);
            if (response.ok) {
                const config = await response.json();
                this.currentShipConfig = config;
                console.log('Ship config loaded:', config);
                document.getElementById('ship-name').textContent = config.ship_name;
                document.getElementById('ship-info-name').textContent = config.ship_name;
                document.getElementById('ship-info-desc').textContent = 
                    `${config.dynasty} · ${config.historical_description}`;

                this.updateComponentsForShip();
            }
        } catch (e) {
            console.error('Failed to load ship config:', e);
        }
    }

    updateComponentsForShip() {
        if (this.extremeSimulator && this.currentShipConfig) {
            this.extremeSimulator.setShip(this.currentShipId, this.currentShipConfig);
        }
        if (this.vrCompartment && this.currentShipConfig) {
            this.vrCompartment.setShip(this.currentShipId, this.currentShipConfig);
        }
    }

    async runSimulation() {
        const compartmentsInput = document.getElementById('damage-compartments').value;
        const severity = parseFloat(document.getElementById('damage-severity').value);

        if (!compartmentsInput.trim()) {
            alert('请输入破损舱室ID');
            return;
        }

        const compartments = compartmentsInput.split(',').map(s => parseInt(s.trim())).filter(n => !isNaN(n));

        if (compartments.length === 0) {
            alert('请输入有效的舱室ID');
            return;
        }

        try {
            const response = await fetch(`${API_BASE}/api/simulate`, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    ship_id: this.currentShipId,
                    flooded_compartments: compartments,
                    damage_severity: severity
                })
            });

            const result = await response.json();
            this.handleSimulationResult(result);
        } catch (e) {
            console.error('Simulation failed:', e);
            alert('仿真失败，请检查后端服务是否运行');
        }
    }

    async runOptimization() {
        const minCompartments = parseInt(document.getElementById('min-compartments').value);
        const maxCompartments = parseInt(document.getElementById('max-compartments').value);

        if (minCompartments >= maxCompartments) {
            alert('最小舱数必须小于最大舱数');
            return;
        }

        const btn = document.getElementById('optimize-btn');
        const originalText = btn.textContent;
        btn.textContent = '优化中...';
        btn.disabled = true;

        try {
            const response = await fetch(`${API_BASE}/api/optimize`, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    ship_id: this.currentShipId,
                    min_compartments: minCompartments,
                    max_compartments: maxCompartments,
                    population_size: 30,
                    generations: 50
                })
            });

            const result = await response.json();
            console.log('Optimization result:', result);

            if (result.configuration && result.configuration.length > 0) {
                this.ship3D.setCompartmentConfiguration(result.configuration);
                alert(`优化完成！\n最优舱数: ${result.compartment_count}\n适应度: ${result.fitness_score.toFixed(4)}\n生存概率: ${(result.survival_probability * 100).toFixed(1)}%`);
            }
        } catch (e) {
            console.error('Optimization failed:', e);
            alert('优化失败，请检查后端服务是否运行');
        } finally {
            btn.textContent = originalText;
            btn.disabled = false;
        }
    }

    resetState() {
        this.alarms = [];
        this.updateAlarmDisplay();
        this.draftHistory = [];
        this.timeLabels = [];
        this.updateDraftChart();
        this.generateInitialStabilityCurve();
        this.initCompartmentList();

        document.getElementById('ship-name').className = 'status-item';

        ['draft-value', 'heel-value', 'trim-value', 'gm-value', 'gz-value', 'buoyancy-value'].forEach(id => {
            document.getElementById(id).className = 'value';
        });

        const draft = this.currentShipConfig?.design_draft || 2.8;
        document.getElementById('draft-value').textContent = `${draft.toFixed(2)} m`;
        document.getElementById('heel-value').textContent = '0.00°';
        document.getElementById('trim-value').textContent = '0.00°';
        document.getElementById('gm-value').textContent = '0.500 m';
        document.getElementById('gm-value').className = 'value success';
        document.getElementById('gz-value').textContent = '0.00 m';
        document.getElementById('buoyancy-value').textContent = '35.0%';
        document.getElementById('buoyancy-value').className = 'value success';
    }

    resetShip() {
        this.ship3D.reset();
        this.resetState();
        document.getElementById('heel-value').textContent = '0.00°';
        document.getElementById('trim-value').textContent = '0.00°';
        document.getElementById('gm-value').textContent = '0.50 m';
        document.getElementById('gz-value').textContent = '0.00 m';
        document.getElementById('buoyancy-value').textContent = '35.0%';

        this.initCompartmentList();
    }

    playDemoAnimation() {
        const compartments = [3, 4];
        const levels = [2.0, 2.5];
        this.ship3D.startFloodingAnimation(compartments, levels, 6000);
    }

    updateConnectionStatus(connected) {
        const el = document.getElementById('connection-status');
        if (connected) {
            el.textContent = '已连接';
            el.className = 'status-item connected';
        } else {
            el.textContent = '未连接';
            el.className = 'status-item warning';
        }
    }

    updateStatusDisplay(data) {
        const draftEl = document.getElementById('draft-value');
        const heelEl = document.getElementById('heel-value');
        const trimEl = document.getElementById('trim-value');
        const gmEl = document.getElementById('gm-value');
        const gzEl = document.getElementById('gz-value');

        draftEl.textContent = `${data.draft?.toFixed(2) || '0.00'} m`;
        heelEl.textContent = `${data.heel_angle?.toFixed(2) || '0.00'}°`;
        trimEl.textContent = `${data.trim_angle?.toFixed(2) || '0.00'}°`;
        gmEl.textContent = `${data.metacentric_height?.toFixed(3) || '0.000'} m`;
        gzEl.textContent = `${data.righting_arm?.toFixed(3) || '0.000'} m`;

        gmEl.className = 'value ' + (data.metacentric_height > 0.3 ? 'success' : data.metacentric_height > 0.15 ? '' : 'danger');
        heelEl.className = 'value ' + (Math.abs(data.heel_angle) < 10 ? '' : 'danger');
        draftEl.className = 'value ' + (data.draft < 4.0 ? '' : 'danger');
    }

    initCompartmentList() {
        const container = document.getElementById('compartment-list');
        const names = this.currentShipConfig?.compartment_names || COMPARTMENT_NAMES;
        container.innerHTML = names.map((name, i) => `
            <div class="compartment-item" id="compartment-${i}">
                <span>${i + 1}. ${name}</span>
                <div class="water-bar">
                    <div class="water-fill" style="width: 0%"></div>
                </div>
                <span style="width: 50px; text-align: right;">0%</span>
            </div>
        `).join('');

        this.updateWaterChartLabels();
    }

    updateWaterChartLabels() {
        if (!this.waterChart || !this.currentShipConfig) return;
        this.waterChart.data.labels = this.currentShipConfig.compartment_names;
        this.waterChart.data.datasets[0].data = new Array(this.currentShipConfig.compartment_count).fill(0);
        this.waterChart.data.datasets[0].backgroundColor = new Array(this.currentShipConfig.compartment_count).fill('rgba(79, 172, 254, 0.6)');
        this.waterChart.update();
    }

    updateCompartmentStates(data) {
        data.forEach(d => {
            const el = document.getElementById(`compartment-${d.compartment_id}`);
            if (!el) return;

            const percentage = Math.min((d.water_level / d.max_water_level) * 100, 100);
            const fillEl = el.querySelector('.water-fill');
            const pctEl = el.querySelector('span:last-child');

            if (fillEl) fillEl.style.width = `${percentage}%`;
            if (pctEl) pctEl.textContent = `${percentage.toFixed(0)}%`;

            if (d.is_flooded || percentage > 30) {
                el.classList.add('flooded');
            } else {
                el.classList.remove('flooded');
            }
        });
    }

    updateStabilityChart(curve) {
        if (!curve || curve.length === 0) return;

        const data = curve.map(p => p.righting_arm);
        this.stabilityChart.data.datasets[0].data = data;

        const maxGz = Math.max(...data);
        document.getElementById('gz-value').textContent = `${maxGz.toFixed(3)} m`;

        this.stabilityChart.update();
    }

    updateWaterChart(data) {
        const waterData = this.waterChart.data.datasets[0].data;
        const colors = this.waterChart.data.datasets[0].backgroundColor;

        data.forEach(d => {
            if (d.compartment_id < waterData.length) {
                waterData[d.compartment_id] = d.water_level;
                colors[d.compartment_id] = d.is_flooded || d.water_level > 1.0
                    ? 'rgba(255, 107, 107, 0.7)'
                    : 'rgba(79, 172, 254, 0.6)';
            }
        });

        this.waterChart.update();
    }

    updateDraftHistory(data) {
        const now = new Date().toLocaleTimeString();
        this.timeLabels.push(now);
        this.draftHistory.push(data.draft);

        if (this.timeLabels.length > 30) {
            this.timeLabels.shift();
            this.draftHistory.shift();
        }

        this.updateDraftChart(data);
    }

    updateDraftChart(data) {
        this.draftChart.data.labels = this.timeLabels;
        this.draftChart.data.datasets[0].data = this.draftHistory;

        if (data) {
            const heelData = this.draftChart.data.datasets[1].data;
            heelData.push(data.heel_angle);
            if (heelData.length > 30) heelData.shift();
        }

        this.draftChart.update();
    }

    initTabs() {
        const tabBtns = document.querySelectorAll('.tab-btn');
        tabBtns.forEach(btn => {
            btn.addEventListener('click', () => {
                const tabId = btn.dataset.tab;
                tabBtns.forEach(b => b.classList.remove('active'));
                btn.classList.add('active');
                document.querySelectorAll('.tab-content').forEach(content => {
                    content.classList.remove('active');
                });
                document.getElementById(`tab-${tabId}`).classList.add('active');
            });
        });
    }

    initShipSelector() {
        const shipSelect = document.getElementById('ship-select');
        shipSelect.addEventListener('change', async () => {
            const shipId = shipSelect.value;
            try {
                const response = await fetch(`/api/ships/${shipId}`);
                if (response.ok) {
                    const shipConfig = await response.json();
                    this.currentShipConfig = shipConfig;
                    this.currentShipId = shipId;
                    document.getElementById('ship-name').textContent = shipConfig.ship_name;
                    document.getElementById('ship-info-name').textContent = shipConfig.ship_name;
                    document.getElementById('ship-info-desc').textContent = 
                        `${shipConfig.dynasty} · ${shipConfig.historical_description}`;
                    this.updateDoorControls();
                    this.initAttackPoints();
                    this.updateComponentsForShip();
                    this.resetState();
                    if (window.ship3D) {
                        window.ship3D.switchShipConfig(shipConfig);
                    }
                }
            } catch (error) {
                console.error('Failed to load ship config:', error);
            }
        });
    }

    initComparison() {
        const compareBtn = document.getElementById('compare-btn');
        compareBtn.addEventListener('click', async () => {
            const checkboxes = document.querySelectorAll('#ship-multi-select input[type="checkbox"]:checked');
            const shipIds = Array.from(checkboxes).map(cb => cb.value);
            
            if (shipIds.length < 2) {
                alert('请至少选择两艘船舶进行对比');
                return;
            }

            try {
                compareBtn.textContent = '对比中...';
                compareBtn.disabled = true;
                const response = await fetch('/api/compare', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ ship_ids: shipIds })
                });

                if (response.ok) {
                    const result = await response.json();
                    this.renderComparisonResults(result);
                }
            } catch (error) {
                console.error('Comparison failed:', error);
            } finally {
                compareBtn.textContent = '开始对比';
                compareBtn.disabled = false;
            }
        });
    }

    renderComparisonResults(result) {
        const resultsDiv = document.getElementById('comparison-results');
        const table = document.getElementById('comparison-table');
        const conclusion = document.getElementById('comparison-conclusion');

        let header = '<thead><tr><th>指标</th>';
        result.ships.forEach(ship => {
            header += `<th>${ship.ship_name.split('（')[0]}</th>`;
        });
        header += '</tr></thead><tbody>';

        result.comparison_metrics.forEach(metric => {
            const values = Object.entries(metric.ship_values);
            const maxVal = Math.max(...values.map(v => v[1]));
            const minVal = Math.min(...values.map(v => v[1]));
            const bestIsMax = metric.name !== '平均舱长';
            
            let row = `<tr><td>${metric.name} (${metric.unit})</td>`;
            result.ships.forEach(ship => {
                const val = metric.ship_values[ship.ship_id];
                const isBest = bestIsMax ? val === maxVal : val === minVal;
                row += `<td class="${isBest ? 'best-value' : ''}">${typeof val === 'number' ? val.toFixed(2) : val}</td>`;
            });
            row += '</tr>';
            header += row;
        });

        header += '</tbody>';
        table.innerHTML = header;
        conclusion.innerHTML = `📌 <strong>分析结论：</strong>${result.conclusion}`;
        resultsDiv.style.display = 'block';
    }

    initEraComparison() {
        const compareBtn = document.getElementById('era-compare-btn');
        compareBtn.addEventListener('click', async () => {
            const ancientId = document.getElementById('ancient-ship-select').value;
            const modernId = document.getElementById('modern-ship-select').value;

            try {
                compareBtn.textContent = '对比中...';
                compareBtn.disabled = true;
                const response = await fetch('/api/compare/era', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ ancient_ship_id: ancientId, modern_ship_id: modernId })
                });

                if (response.ok) {
                    const result = await response.json();
                    this.renderEraComparisonResults(result);
                }
            } catch (error) {
                console.error('Era comparison failed:', error);
            } finally {
                compareBtn.textContent = '开始跨时代对比';
                compareBtn.disabled = false;
            }
        });
    }

    renderEraComparisonResults(result) {
        const resultsDiv = document.getElementById('era-results');
        
        document.getElementById('era-timeline').textContent = result.era_comparison.timeline;
        document.getElementById('era-design').innerHTML = `
            <strong>设计哲学：</strong>${result.era_comparison.design_philosophy_difference}<br><br>
            <strong>技术演进：</strong>${result.era_comparison.technology_evolution}<br><br>
            <strong>规范体系：</strong>${result.era_comparison.regulatory_framework}
        `;

        const simResultsDiv = document.getElementById('era-simulation-results');
        simResultsDiv.innerHTML = '';
        
        result.simulation_results.forEach(sim => {
            const ancientSafe = sim.ancient_result.is_safe;
            const modernSafe = sim.modern_result.is_safe;
            
            const div = document.createElement('div');
            div.className = 'simulation-result-row';
            div.innerHTML = `
                <span>${sim.scenario}</span>
                <span class="winner">🏆 ${sim.winner}</span>
                <span style="color: ${ancientSafe ? '#2ed573' : '#ff6b6b'};">古船: ${ancientSafe ? '安全' : '沉没'}</span>
                <span style="color: ${modernSafe ? '#2ed573' : '#ff6b6b'};">现代: ${modernSafe ? '安全' : '沉没'}</span>
            `;
            simResultsDiv.appendChild(div);

            const analysisDiv = document.createElement('div');
            analysisDiv.style.cssText = 'font-size: 11px; color: #aaa; margin-top: 4px; margin-bottom: 8px;';
            analysisDiv.textContent = sim.analysis;
            simResultsDiv.appendChild(analysisDiv);
        });

        document.getElementById('era-conclusion').innerHTML = `
            <strong>📜 跨时代对比总结：</strong><br>
            从${result.ancient_ship.dynasty}的「${result.ancient_ship.ship_name}」到现代的「${result.modern_ship.ship_name}」，
            水密隔舱技术跨越千年，核心思想一脉相承。古代工匠凭借经验创造的抗沉设计，与现代基于SOLAS公约的科学分舱，
            共同见证了人类航海技术的伟大传承与发展。
        `;

        resultsDiv.style.display = 'block';
    }

    initPirateAttack() {
        this.attackPoints = [];
        this.initAttackPoints();

        document.getElementById('add-attack-btn').addEventListener('click', () => {
            this.addAttackPoint();
        });

        document.getElementById('pirate-attack-btn').addEventListener('click', async () => {
            if (this.attackPoints.length === 0) {
                alert('请至少添加一个攻击点');
                return;
            }

            const duration = parseInt(document.getElementById('attack-duration').value);
            const payload = {
                ship_id: this.currentShipId,
                attack_points: this.attackPoints,
                simulation_duration_seconds: duration
            };

            try {
                const btn = document.getElementById('pirate-attack-btn');
                btn.textContent = '仿真中...';
                btn.disabled = true;
                const response = await fetch('/api/simulate/pirate', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify(payload)
                });

                if (response.ok) {
                    const result = await response.json();
                    this.renderPirateAttackResults(result);
                    this.handleSimulationResult(result.final_state);
                    if (window.ship3D) {
                        window.ship3D.showPirateAttackEffect();
                    }
                }
            } catch (error) {
                console.error('Pirate attack simulation failed:', error);
            } finally {
                const btn = document.getElementById('pirate-attack-btn');
                btn.textContent = '开始海盗攻击仿真';
                btn.disabled = false;
            }
        });
    }

    initAttackPoints() {
        this.attackPoints = [];
        const container = document.getElementById('attack-points');
        container.innerHTML = '';
        
        if (this.currentShipConfig) {
            const numCompartments = this.currentShipConfig.compartment_count;
            this.addAttackPoint(Math.floor(numCompartments / 3), 0.7, 0);
            this.addAttackPoint(Math.floor(numCompartments / 2), 0.8, 60);
        }
    }

    addAttackPoint(compartmentId = 3, severity = 0.7, delay = 0) {
        const container = document.getElementById('attack-points');
        const index = this.attackPoints.length;
        
        const maxCompartment = this.currentShipConfig ? this.currentShipConfig.compartment_count - 1 : 12;

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
            damage_severity: severity,
            delay_seconds: delay
        });

        div.querySelector('.remove-attack-btn').addEventListener('click', (e) => {
            const idx = parseInt(e.target.dataset.index);
            this.attackPoints.splice(idx, 1);
            div.remove();
            this.updateAttackPointIndices();
        });

        div.querySelectorAll('input').forEach(input => {
            input.addEventListener('change', () => {
                this.updateAttackPointsFromDOM();
            });
        });
    }

    updateAttackPointIndices() {
        document.querySelectorAll('.remove-attack-btn').forEach((btn, idx) => {
            btn.dataset.index = idx;
        });
    }

    updateAttackPointsFromDOM() {
        const rows = document.querySelectorAll('.attack-point-row');
        this.attackPoints = [];
        rows.forEach(row => {
            const comp = parseInt(row.querySelector('.attack-compartment').value);
            const sev = parseFloat(row.querySelector('.attack-severity').value);
            const delay = parseInt(row.querySelector('.attack-delay').value);
            if (!isNaN(comp) && !isNaN(sev) && !isNaN(delay)) {
                this.attackPoints.push({
                    compartment_id: comp,
                    damage_severity: sev,
                    delay_seconds: delay
                });
            }
        });
    }

    renderPirateAttackResults(result) {
        const resultsDiv = document.getElementById('pirate-results');
        const indicator = document.getElementById('survival-indicator');
        const percent = document.getElementById('survival-percent');
        const timeline = document.getElementById('attack-timeline');
        const critical = document.getElementById('critical-moments');

        const survivalPct = (result.survival_probability * 100).toFixed(1);
        percent.textContent = `${survivalPct}%`;
        
        if (result.survival_probability < 0.3) {
            indicator.classList.add('danger');
        } else {
            indicator.classList.remove('danger');
        }

        timeline.innerHTML = '';
        result.timeline_events.forEach(event => {
            const div = document.createElement('div');
            div.className = `timeline-event ${event.event_type === 'UNSAFE' ? 'critical' : event.event_type === 'SURVIVED' ? 'safe' : ''}`;
            div.innerHTML = `
                <div class="time">${event.time_seconds.toFixed(0)}s</div>
                <div class="desc">${event.description}</div>
                <div style="font-size: 10px; color: #888; margin-top: 2px;">
                    进水舱: ${event.affected_compartments.join(', ')} | GM: ${event.gm_value.toFixed(3)}m | ${event.is_safe ? '✅安全' : '❌危险'}
                </div>
            `;
            timeline.appendChild(div);
        });

        critical.innerHTML = '';
        if (result.critical_moments.length > 0) {
            result.critical_moments.forEach(moment => {
                const div = document.createElement('div');
                div.style.cssText = 'padding: 8px; background: rgba(255, 107, 107, 0.1); border-left: 3px solid #ff6b6b; border-radius: 4px; margin-bottom: 5px; font-size: 11px;';
                div.innerHTML = `
                    <strong>${moment.time_seconds.toFixed(0)}s</strong> - ${moment.description}
                    <div style="color: #ff6b6b; font-size: 10px; margin-top: 2px;">GM = ${moment.gm_value.toFixed(3)}m</div>
                `;
                critical.appendChild(div);
            });
        } else {
            critical.innerHTML = '<div style="color: #2ed573; font-size: 11px; text-align: center; padding: 10px;">✅ 无危险时刻，船舶状态良好</div>';
        }

        resultsDiv.style.display = 'block';
    }

    initDoorControls() {
        this.doorStates = {};
        this.hapticEnabled = navigator.vibrate !== undefined;
        this.audioContext = null;
        this.updateDoorControls();

        document.getElementById('interactive-simulate-btn').addEventListener('click', async () => {
            const damageInput = document.getElementById('interactive-damage').value;
            const severity = parseFloat(document.getElementById('interactive-severity').value);

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

            try {
                const btn = document.getElementById('interactive-simulate-btn');
                btn.textContent = '仿真中...';
                btn.disabled = true;
                const response = await fetch('/api/simulate/interactive', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify(payload)
                });

                if (response.ok) {
                    const result = await response.json();
                    this.handleSimulationResult(result);
                    this.renderInteractiveResult(result, doorStatesArray);
                }
            } catch (error) {
                console.error('Interactive simulation failed:', error);
                this.showToast('仿真失败，请重试', 'warning');
            } finally {
                const btn = document.getElementById('interactive-simulate-btn');
                btn.textContent = '基于当前舱门状态仿真';
                btn.disabled = false;
            }
        });
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

            div.querySelector('input').addEventListener('change', async (e) => {
                const bulkheadId = parseInt(e.target.dataset.bulkhead);
                const isOpen = e.target.checked;
                const isClosing = !isOpen;

                this.doorStates[bulkheadId] = isOpen;

                const statusEl = document.getElementById(`door-status-${bulkheadId}`);
                statusEl.textContent = isOpen ? '开启' : '关闭';
                statusEl.className = `door-status ${isOpen ? 'open' : 'closed'}`;

                const controlEl = document.getElementById(`door-control-${bulkheadId}`);
                const switchEl = document.getElementById(`switch-${bulkheadId}`);

                controlEl.classList.remove('active', 'door-locking', 'door-unlocking', 'force-feedback-pulse');
                void controlEl.offsetWidth;

                if (isClosing) {
                    controlEl.classList.add('active', 'door-locking');
                    switchEl.classList.add('force-feedback-pulse');
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

                try {
                    await fetch('/api/door', {
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
            });
        }
    }

    renderInteractiveResult(result, doorStates) {
        const resultsDiv = document.getElementById('interactive-result');
        const conclusion = document.getElementById('interactive-conclusion');
        
        const openDoors = doorStates.filter(d => d.is_open).length;
        const totalDoors = doorStates.length;
        const floodedCount = result.flooded_compartments.length;

        let message = '';
        if (result.is_safe) {
            message = `✅ <strong>船舶保持安全！</strong><br><br>`;
        } else {
            message = `❌ <strong>船舶处于危险状态！</strong><br><br>`;
        }

        message += `
            <strong>操作分析：</strong>您开启了 ${openDoors}/${totalDoors} 道舱门，初始破损 ${result.flooded_compartments.length} 个舱室。<br><br>
            <strong>仿真结果：</strong><br>
            • 最终进水舱室: ${floodedCount} 个<br>
            • 最终吃水: ${result.final_draft.toFixed(2)} m<br>
            • 初稳心高 GM: ${result.metacentric_height.toFixed(3)} m<br>
            • 横倾角: ${result.final_heel_angle.toFixed(1)}°<br>
            • 储备浮力: ${result.reserve_buoyancy.toFixed(1)}%<br><br>
            <strong>教育提示：</strong>${openDoors > 0 ? 
                '关闭水密舱门可以有效阻止进水蔓延！水密隔舱的抗沉效果取决于舱门是否保持水密。' : 
                '所有舱门紧闭，水密隔舱发挥了最大效能！这展示了水密隔舱技术的核心原理——通过封闭舱室限制进水范围。'}
        `;

        conclusion.innerHTML = message;
        resultsDiv.style.display = 'block';
    }
}

document.addEventListener('DOMContentLoaded', () => {
    new FloodingPanel();
});
