function escapeHtml(value) {
  return String(value == null ? '' : value)
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
    .replaceAll('"', '&quot;')
    .replaceAll("'", '&#39;');
}

function ensureDataset(element) {
  if (!element) return null;
  if (!element.dataset) {
    element.dataset = {};
  }
  return element.dataset;
}

function setDataAttributes(element, attributes) {
  const dataset = ensureDataset(element);
  if (!dataset) return;
  for (const [key, value] of Object.entries(attributes)) {
    dataset[key] = String(value);
  }
}

function applyChipState(element, { label, tone, panelId, statusKey, statusValue }) {
  if (!element) return;
  element.textContent = label;
  element.className = `pill risk-pill ${tone}`;
  setDataAttributes(element, {
    panel: panelId,
    [statusKey]: statusValue
  });
}

export function renderRevenueTruthSurface(elements, viewModel) {
  applyChipState(elements.revenueTruthGuardChip, {
    label: viewModel.guardLabel,
    tone: viewModel.guardTone,
    panelId: viewModel.panelId,
    statusKey: 'guardStatus',
    statusValue: viewModel.guardStatus
  });
  applyChipState(elements.revenueTruthRiskChip, {
    label: viewModel.riskLabel,
    tone: viewModel.riskTone,
    panelId: viewModel.panelId,
    statusKey: 'riskLevel',
    statusValue: viewModel.risk
  });

  if (!elements.revenueTruthPanel) return;

  const metricsMarkup = viewModel.metrics
    .map(
      (metric) => `
      <div class="report-metric" data-metric-key="${escapeHtml(metric.key)}" data-metric-value="${escapeHtml(metric.rawValue)}">
        <div class="forecast-label">${escapeHtml(metric.label)}</div>
        <div class="forecast-value">${escapeHtml(metric.displayValue)}</div>
      </div>`
    )
    .join('');

  elements.revenueTruthPanel.innerHTML = `
    <div class="report-metrics" data-panel="revenue-truth-metrics">
      ${metricsMarkup}
    </div>
    <div class="narrative-item" data-section="summary">${escapeHtml(viewModel.summary)}</div>
  `;
  setDataAttributes(elements.revenueTruthPanel, {
    panel: viewModel.panelId,
    guardStatus: viewModel.guardStatus,
    riskLevel: viewModel.risk,
    metricCount: viewModel.metrics.length
  });
}

export function renderPublishGateSurface(elements, viewModel) {
  if (elements.exportPacketButton) {
    elements.exportPacketButton.disabled = !viewModel.exportReady;
    elements.exportPacketButton.title = viewModel.exportReady
      ? 'Export gate is open for this snapshot.'
      : `Blocked: ${viewModel.blockingReasons.join(' | ') || 'publish/export gate failed'}`;
    setDataAttributes(elements.exportPacketButton, {
      panel: viewModel.panelId,
      gateStatus: viewModel.gateStatus,
      exportReady: viewModel.exportReady
    });
  }

  if (!elements.publishGatePanel) return;

  elements.publishGatePanel.innerHTML = viewModel.sections
    .map((section) => {
      if (section.key === 'status') {
        return `
          <div class="gate-card" data-section="${escapeHtml(section.key)}">
            <h3>${escapeHtml(section.title)}</h3>
            <div class="gate-status ${escapeHtml(viewModel.gateStatus)}" data-gate-status="${escapeHtml(viewModel.gateStatus)}">${escapeHtml(viewModel.gateStatus.replaceAll('_', ' '))}</div>
            <p>${escapeHtml(section.bodyLines[0])}</p>
            <p>${escapeHtml(section.bodyLines[1])}</p>
          </div>`;
      }
      return `
        <div class="gate-card" data-section="${escapeHtml(section.key)}">
          <h3>${escapeHtml(section.title)}</h3>
          <p>${escapeHtml(section.bodyLines[0] || 'None')}</p>
        </div>`;
    })
    .join('');

  setDataAttributes(elements.publishGatePanel, {
    panel: viewModel.panelId,
    gateStatus: viewModel.gateStatus,
    publishReady: viewModel.publishReady,
    exportReady: viewModel.exportReady,
    blockingCount: viewModel.blockingReasons.length,
    warningCount: viewModel.warningReasons.length
  });
}

export function renderDecisionFeedSurface(elements, viewModel) {
  if (!elements.decisionFeedList) return;

  elements.decisionFeedList.innerHTML = viewModel.cards
    .map(
      (card) => `
    <div class="decision-card ${escapeHtml(card.priority)}" data-card-id="${escapeHtml(card.cardId)}" data-priority="${escapeHtml(card.priority)}" data-status="${escapeHtml(card.status)}">
      <div class="decision-meta">${escapeHtml(card.priority)} | ${escapeHtml(card.status)}</div>
      <h3>${escapeHtml(card.title)}</h3>
      <p>${escapeHtml(card.summary)}</p>
      <p><strong>Action:</strong> ${escapeHtml(card.recommendedAction)}</p>
    </div>`
    )
    .join('');

  setDataAttributes(elements.decisionFeedList, {
    panel: viewModel.panelId,
    cardCount: viewModel.cards.length
  });
}

export function renderKpiSurface(elements, viewModel) {
  if (!elements.kpiGrid) return;

  elements.kpiGrid.innerHTML = viewModel.cards
    .map((card) => {
      if (!card.tooltipId) {
        return `<div class="kpi" data-kpi-key="${escapeHtml(card.key)}">
          <div class="kpi-label">${escapeHtml(card.label)}</div>
          <div class="kpi-value">${escapeHtml(card.displayValue)}</div>
          <div class="kpi-note">${escapeHtml(card.note)}</div>
        </div>`;
      }

      return `<div class="kpi" data-kpi-key="${escapeHtml(card.key)}" data-confidence-label="${escapeHtml(card.confidenceLabel || 'unknown')}">
        <div class="kpi-head">
          <div class="kpi-label">${escapeHtml(card.label)}</div>
          <button
            type="button"
            class="kpi-info"
            data-field="info-button"
            aria-label="How ${escapeHtml(card.label || 'this KPI')} is calculated"
            aria-describedby="${escapeHtml(card.tooltipId)}"
          >?</button>
        </div>
        <div class="kpi-value">${escapeHtml(card.displayValue)}</div>
        <div class="kpi-note">${escapeHtml(card.note)}</div>
        <div id="${escapeHtml(card.tooltipId)}" class="kpi-tooltip" data-field="tooltip" role="tooltip">${escapeHtml(card.tooltipText || '')}</div>
      </div>`;
    })
    .join('');

  setDataAttributes(elements.kpiGrid, {
    panel: viewModel.panelId,
    kpiCount: viewModel.cards.length
  });
}

export function renderChartSurfaceMetadata(element, diagnostics) {
  if (!element || !diagnostics) return;
  setDataAttributes(element, {
    chartKey: diagnostics.chartKey || 'unknown',
    pointCount: diagnostics.pointCount || 0,
    datasetCount: diagnostics.datasetCount || 0
  });
}

export function renderChartSummarySurface(element, { chartKey, summaryText, diagnostics }) {
  if (element && typeof summaryText === 'string') {
    element.textContent = summaryText;
  }
  if (element && diagnostics) {
    setDataAttributes(element, {
      chartKey: chartKey || diagnostics.chartKey || 'unknown',
      pointCount: diagnostics.pointCount || 0,
      datasetCount: diagnostics.datasetCount || 0
    });
  }
}
