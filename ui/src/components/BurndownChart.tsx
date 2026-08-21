import { useState, useEffect, useCallback } from "react";
import { useTauri } from "../hooks/useTauri";

interface BurndownPoint {
  date: string;
  remaining: number;
}

interface BurndownData {
  total_points: number;
  points: BurndownPoint[];
}

interface BurndownChartProps {
  sprintId: string;
}

const SVG_WIDTH = 700;
const SVG_HEIGHT = 320;
const PADDING = { top: 30, right: 30, bottom: 50, left: 55 };

export default function BurndownChart({ sprintId }: BurndownChartProps) {
  const tauri = useTauri();
  const [data, setData] = useState<BurndownData | null>(null);
  const [loading, setLoading] = useState(true);
  const [tooltip, setTooltip] = useState<{
    x: number;
    y: number;
    date: string;
    remaining: number;
    ideal: number;
  } | null>(null);

  useEffect(() => {
    const load = async () => {
      try {
        const result = await tauri.getSprintBurndown(sprintId);
        setData(result);
      } catch (err) {
        console.error("Failed to load burndown:", err);
      } finally {
        setLoading(false);
      }
    };
    load();
  }, [sprintId, tauri]);

  if (loading || !data || data.points.length === 0) {
    return (
      <div className="chart-placeholder">
        <p>No burndown data yet</p>
      </div>
    );
  }

  const { total_points, points } = data;
  const chartWidth = SVG_WIDTH - PADDING.left - PADDING.right;
  const chartHeight = SVG_HEIGHT - PADDING.top - PADDING.bottom;

  const maxPoints = total_points;
  const numDays = points.length;

  const scaleX = (i: number) => PADDING.left + (i / (numDays - 1 || 1)) * chartWidth;
  const scaleY = (val: number) =>
    PADDING.top + chartHeight - (val / (maxPoints || 1)) * chartHeight;

  // Build ideal line path (straight from total_points to 0)
  const idealPath = `M ${scaleX(0)} ${scaleY(total_points)} L ${scaleX(numDays - 1)} ${scaleY(0)}`;

  // Build actual line path
  const actualPath = points
    .map((p, i) => `${i === 0 ? "M" : "L"} ${scaleX(i)} ${scaleY(p.remaining)}`)
    .join(" ");

  // Build area under actual
  const areaPath = `${actualPath} L ${scaleX(numDays - 1)} ${scaleY(0)} L ${scaleX(0)} ${scaleY(0)} Z`;

  // X-axis labels (show every Nth day)
  const labelInterval = Math.max(1, Math.ceil(numDays / 7));
  const xLabels = points
    .filter((_, i) => i % labelInterval === 0 || i === numDays - 1)
    .map((p, _, arr) => {
      const i = points.indexOf(p);
      return { x: scaleX(i), label: p.date.slice(5) }; // MM-DD
    });

  // Y-axis labels
  const yLabelCount = 5;
  const yLabels = Array.from({ length: yLabelCount + 1 }, (_, i) => {
    const val = Math.round((maxPoints / yLabelCount) * i);
    return { y: scaleY(val), label: val.toString() };
  });

  return (
    <div className="burndown-chart">
      <h4>Burndown</h4>
      <svg
        width={SVG_WIDTH}
        height={SVG_HEIGHT}
        viewBox={`0 0 ${SVG_WIDTH} ${SVG_HEIGHT}`}
      >
        {/* Grid lines */}
        {yLabels.map((yl, i) => (
          <line
            key={i}
            x1={PADDING.left}
            y1={yl.y}
            x2={SVG_WIDTH - PADDING.right}
            y2={yl.y}
            stroke="var(--border)"
            strokeWidth={0.5}
          />
        ))}

        {/* Y-axis labels */}
        {yLabels.map((yl, i) => (
          <text
            key={i}
            x={PADDING.left - 10}
            y={yl.y + 4}
            textAnchor="end"
            fill="var(--text-muted)"
            fontSize={11}
          >
            {yl.label}
          </text>
        ))}

        {/* X-axis labels */}
        {xLabels.map((xl, i) => (
          <text
            key={i}
            x={xl.x}
            y={SVG_HEIGHT - PADDING.bottom + 20}
            textAnchor="middle"
            fill="var(--text-muted)"
            fontSize={10}
          >
            {xl.label}
          </text>
        ))}

        {/* Ideal line */}
        <path d={idealPath} stroke="#656d76" strokeWidth={1.5} strokeDasharray="6,4" fill="none" />

        {/* Area under actual */}
        <path d={areaPath} fill="rgba(88,166,255,0.08)" />

        {/* Actual line */}
        <path d={actualPath} stroke="var(--accent)" strokeWidth={2} fill="none" strokeLinejoin="round" />

        {/* Actual line dots */}
        {points.map((p, i) => (
          <circle
            key={i}
            cx={scaleX(i)}
            cy={scaleY(p.remaining)}
            r={3}
            fill="var(--accent)"
            style={{ cursor: "pointer" }}
            onMouseEnter={(e) => {
              const ideal = Math.round(total_points * (1 - i / (numDays - 1 || 1)));
              setTooltip({
                x: e.clientX,
                y: e.clientY,
                date: p.date,
                remaining: p.remaining,
                ideal,
              });
            }}
            onMouseLeave={() => setTooltip(null)}
          />
        ))}
      </svg>

      {/* Legend */}
      <div className="chart-legend">
        <span className="chart-legend-item">
          <span className="chart-legend-line" style={{ borderTop: "2px dashed #656d76" }} />
          Ideal
        </span>
        <span className="chart-legend-item">
          <span className="chart-legend-line" style={{ borderTop: "2px solid var(--accent)" }} />
          Actual
        </span>
      </div>

      {/* Tooltip */}
      {tooltip && (
        <div
          className="chart-tooltip"
          style={{
            position: "fixed",
            left: tooltip.x + 12,
            top: tooltip.y - 10,
          }}
        >
          <div>{tooltip.date}</div>
          <div>
            Remaining: <strong>{tooltip.remaining}</strong> pts
          </div>
          <div>
            Ideal: <strong>{tooltip.ideal}</strong> pts
          </div>
        </div>
      )}
    </div>
  );
}
