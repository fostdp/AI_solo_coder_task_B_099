export class EraComparator {
    constructor(options = {}) {
        this.apiBase = options.apiBase || '/api';
        this.ancientShips = options.ancientShips || [];
        this.modernShips = options.modernShips || [];
        this.onResult = options.onResult || null;
    }

    init() {
        const compareBtn = document.getElementById('era-compare-btn');
        if (compareBtn) {
            compareBtn.addEventListener('click', () => this.handleCompare());
        }
        this.renderSelectors();
    }

    setShips(ancientShips, modernShips) {
        this.ancientShips = ancientShips;
        this.modernShips = modernShips;
        this.renderSelectors();
    }

    renderSelectors() {
        const ancientSelect = document.getElementById('ancient-ship-select');
        const modernSelect = document.getElementById('modern-ship-select');

        if (ancientSelect) {
            ancientSelect.innerHTML = '';
            this.ancientShips.forEach(ship => {
                const option = document.createElement('option');
                option.value = ship.ship_id;
                option.textContent = ship.ship_name;
                ancientSelect.appendChild(option);
            });
        }

        if (modernSelect) {
            modernSelect.innerHTML = '';
            this.modernShips.forEach(ship => {
                const option = document.createElement('option');
                option.value = ship.ship_id;
                option.textContent = ship.ship_name;
                modernSelect.appendChild(option);
            });
        }
    }

    async handleCompare() {
        const ancientId = document.getElementById('ancient-ship-select')?.value;
        const modernId = document.getElementById('modern-ship-select')?.value;

        if (!ancientId || !modernId) {
            alert('请选择古代和现代船舶');
            return;
        }

        const compareBtn = document.getElementById('era-compare-btn');
        try {
            compareBtn.textContent = '对比中...';
            compareBtn.disabled = true;

            const response = await fetch(`${this.apiBase}/compare/era`, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    ancient_ship_id: ancientId,
                    modern_ship_id: modernId
                })
            });

            if (response.ok) {
                const result = await response.json();
                this.renderResults(result);
                if (this.onResult) {
                    this.onResult(result);
                }
            }
        } catch (error) {
            console.error('Era comparison failed:', error);
        } finally {
            compareBtn.textContent = '开始跨时代对比';
            compareBtn.disabled = false;
        }
    }

    renderResults(result) {
        const resultsDiv = document.getElementById('era-results');
        if (!resultsDiv) return;

        const timelineEl = document.getElementById('era-timeline');
        const designEl = document.getElementById('era-design');
        const simResultsDiv = document.getElementById('era-simulation-results');
        const conclusionEl = document.getElementById('era-conclusion');

        if (timelineEl) {
            timelineEl.textContent = result.era_comparison?.timeline || '';
        }

        if (designEl && result.era_comparison) {
            designEl.innerHTML = `
                <strong>设计哲学：</strong>${result.era_comparison.design_philosophy_difference || ''}<br><br>
                <strong>技术演进：</strong>${result.era_comparison.technology_evolution || ''}<br><br>
                <strong>规范体系：</strong>${result.era_comparison.regulatory_framework || ''}
            `;
        }

        if (simResultsDiv) {
            simResultsDiv.innerHTML = '';
            (result.simulation_results || []).forEach(sim => {
                const ancientSafe = sim.ancient_result?.is_safe;
                const modernSafe = sim.modern_result?.is_safe;

                const div = document.createElement('div');
                div.className = 'simulation-result-row';
                div.innerHTML = `
                    <span>${sim.scenario || ''}</span>
                    <span class="winner">🏆 ${sim.winner || ''}</span>
                    <span style="color: ${ancientSafe ? '#2ed573' : '#ff6b6b'};">古船: ${ancientSafe ? '安全' : '沉没'}</span>
                    <span style="color: ${modernSafe ? '#2ed573' : '#ff6b6b'};">现代: ${modernSafe ? '安全' : '沉没'}</span>
                `;
                simResultsDiv.appendChild(div);

                if (sim.analysis) {
                    const analysisDiv = document.createElement('div');
                    analysisDiv.style.cssText = 'font-size: 11px; color: #aaa; margin-top: 4px; margin-bottom: 8px;';
                    analysisDiv.textContent = sim.analysis;
                    simResultsDiv.appendChild(analysisDiv);
                }
            });
        }

        if (conclusionEl) {
            conclusionEl.innerHTML = `
                <strong>📜 跨时代对比总结：</strong><br>
                从${result.ancient_ship?.dynasty || '古代'}的「${result.ancient_ship?.ship_name || ''}」到现代的「${result.modern_ship?.ship_name || ''}」，
                水密隔舱技术跨越千年，核心思想一脉相承。古代工匠凭借经验创造的抗沉设计，与现代基于SOLAS公约的科学分舱，
                共同见证了人类航海技术的伟大传承与发展。
            `;
        }

        resultsDiv.style.display = 'block';
    }
}

