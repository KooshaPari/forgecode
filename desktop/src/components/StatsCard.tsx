import React from 'react';
import { FiTrendingUp, FiTrendingDown, FiMinus } from 'react-icons/fi';

interface StatsCardProps {
  label: string;
  value: number;
  trend?: 'up' | 'down' | 'neutral';
  accentColor?: string;
}

export default function StatsCard({ label, value, trend = 'neutral', accentColor }: StatsCardProps) {
  const trendIcon = trend === 'up'
    ? <FiTrendingUp size={14} className="trend-up" />
    : trend === 'down'
    ? <FiTrendingDown size={14} className="trend-down" />
    : <FiMinus size={14} className="trend-neutral" />;

  return (
    <div className="stats-card" style={accentColor ? { borderTopColor: accentColor } : undefined}>
      <div className="stats-card-header">
        <span className="stats-label">{label}</span>
        {trendIcon}
      </div>
      <div className="stats-value" style={accentColor ? { color: accentColor } : undefined}>
        {value}
      </div>
    </div>
  );
}
