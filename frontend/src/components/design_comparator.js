export class DesignComparator {
    constructor(options = {}) {
        this.apiBase = options.apiBase || '/api';
        this.ships = options.ships || [];
        this.onResult = options.onResult || null;
    }

    init() {
        const compareBtn = document.getElementById('compare-btn');
        if (compareBtn) {
            compareBtn.addEventListener('click', () => this.handleCompare());
        }
    }

    setShips(ships) {
        this.ships = ships;
        this.renderShipSelector();
    }

    renderShipSelector() {
        const container = document.getElementById('ship-multi-select');
        if (!container) return;

        container.innerHTML = '';
        this.ships.forEach(ship => {
            const label = document.createElement('label');
            label.style.display = 'block';
            label.style.padding = '4px 8px';
            label.style.cursor = 'pointer';
            label.innerHTML = `
                <input type="checkbox" value="${ship.ship_id}" />
                <span style="margin-left: 6px;">${ship.ship_name}</span>
            `;
            container.appendChild(label);
        });
    }

    async handleCompare() {
        const checkboxes = document.querySelectorAll('#ship-multi-select input[type="checkbox"]:checked');
        const shipIds = Array.from(checkboxes).map(cb => cb.value);

        if (shipIds.length < 2) {
            alert('请至少选择两艘船舶进行对比');
            return;
        }

        const compareBtn = document.getElementById('compare-btn');
        try {
            compareBtn.textContent = '对比中...';
            compareBtn.disabled = true;

            const response = await fetch(`${this.apiBase}/compare`, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ ship_ids: shipIds })
            });

            if (response.ok) {
                const result = await response.json();
                this.renderResults(result);
                if (this.onResult) {
                    this.onResult(result);
                }
            }
        } catch (error) {
            console.error('Comparison failed:', error);
        } finally {
            compareBtn.textContent = '开始对比';
            compareBtn.disabled = false;
        }
    }

    renderResults(result) {
        const resultsDiv = document.getElementById('comparison-results');
        const table = document.getElementById('comparison-table');
        const conclusion = document.getElementById('comparison-conclusion');

        if (!resultsDiv || !table || !conclusion) return;

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
}

