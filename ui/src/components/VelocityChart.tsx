interface VelocityPoint {
  sprint_id: string;
  sprint_name: string;
  total_points: number;
  completed_points: number;
}

interface VelocityChartProps {
  data: VelocityPoint[];
  avgVelocity: number;
}

const SVG_WIDTH = 700;
const SVG_HEIGHT = 280;
const PADDING = { top: 30, right: 30, bottom: 50, left: 55 };

export default function VelocityChart({ data, avgVelocity }: VelocityChartProps) {
  if (data.length === 0) {
    return (
      <div className="chart-placeholder">
        <p>No velocity data yet. Complete sprints to see velocity trends.</p>
      </div>
    );
  }

  const chartWidth = SVG_WIDTH - PADDING.left - PADDING.right;
  const chartHeight = SVG_HEIGHT - PADDING.top - PADDING.bottom;

  const maxVal = Math.max(
    ...data.map((d) => Math.max(d.total_points, d.completed_points)),
    avgVelocity,
    1
  );

  const scaleY = (val: number) =>
    PADDING.top + chartHeight - (val / (maxVal || 1)) * chartHeight;

  const barGroupWidth = chartWidth / data.length;
  const barWidth = Math.min(barGroupWidth * 0.35, 40);
  const barGap = 6;

  // Average line
  const avgY = scaleY(avgVelocity);

  return (
    <div className="velocity-chart">
      <h4>Velocity</h4>
      <svg
        width={SVG_WIDTH}
        height={SVG_HEIGHT}
        viewBox={`0 0 ${SVG_WIDTH} ${SVG_HEIGHT}`}
      >
        {/* Grid lines */}
        {[0, 0.25, 0.5, 0.75, 1].map((frac, i) => {
          const y = PADDING.top + chartHeight * (1 - frac);
          const val = Math.round(maxVal * frac);
          return (
            <g key={i}>
              <line
                x1={PADDING.left}
                y1={y}
                x2={SVG_WIDTH - PADDING.right}
                y2={y}
                stroke="var(--border)"
                strokeWidth={0.5}
              />
              <text
                x={PADDING.left - 10}
                y={y + 4}
                textAnchor="end"
                fill="var(--text-muted)"
                fontSize={11}
              >
                {val}
              </text>
            </g>
          );
        })}

        {/* Average line */}
        <line
          x1={PADDING.left}
          y1={avgY}
          x2={SVG_WIDTH - PADDING.right}
          y2={avgY}
          stroke="#d29922"
          strokeWidth={1.5}
          strokeDasharray="6,4"
        />
        <text
          x={SVG_WIDTH - PADDING.right + 4}
          y={avgY + 4}
          fill="#d29922"
          fontSize={10}
        >
          avg
        </text>

        {/* Bars */}
        {data.map((d, i) => {
          const groupX = PADDING.left + i * barGroupWidth + barGroupWidth / 2;
          const totalBarHeight = (d.total_points / (maxVal || 1)) * chartHeight;
          const completedBarHeight = (d.completed_points / (maxVal || 1)) * chartHeight;

          return (
            <g key={d.sprint_id}>
              {/* Total points bar (background) */}
              <rect
                x={groupX - barWidth - barGap / 2}
                y={PADDING.top + chartHeight - totalBarHeight}
                width={barWidth}
                height={totalBarHeight}
                fill="rgba(99,102,241,0.25)"
                rx={3}
              />
              {/* Completed points bar */}
              <rect
                x={groupX + barGap / 2}
                y={PADDING.top + chartHeight - completedBarHeight}
                width={barWidth}
                height={completedBarHeight}
                fill="var(--success)"
                rx={3}
              />
              {/* Sprint name */}
              <text
                x={groupX}
                y={SVG_HEIGHT - PADDING.bottom + 18}
                textAnchor="middle"
                fill="var(--text-muted)"
                fontSize={10}
              >
                {d.sprint_name.length > 10
                  ? d.sprint_name.slice(0, 10) + "..."
                  : d.sprint_name}
              </text>
            </g>
          );
        })}
      </svg>

      {/* Legend */}
      <div className="chart-legend">
        <span className="chart-legend-item">
          <span
            className="chart-legend-swatch"
            style={{ background: "rgba(99,102,241,0.25)" }}
          />
          Total Points
        </span>
        <span className="chart-legend-item">
          <span
            className="chart-legend-swatch"
            style={{ background: "var(--success)" }}
          />
          Completed
        </span>
        <span className="chart-legend-item">
          <span
            className="chart-legend-line"
            style={{ borderTop: "2px dashed #d29922" }}
          />
          Average ({avgVelocity} pts)
        </span>
      </div>
    </div>
  );
}
