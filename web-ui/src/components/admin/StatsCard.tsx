/**
 * StatsCard Component
 *
 * Displays a single statistic card
 */

interface StatsCardProps {
  title: string;
  value: string | number;
  icon: string;
  bgColor?: string;
}

export function StatsCard({ title, value, icon, bgColor = 'bg-blue-500' }: StatsCardProps) {
  return (
    <div className="bg-white dark:bg-gray-800 rounded-lg shadow p-6">
      <div className="flex items-center">
        <div className={`${bgColor} rounded-lg p-3 text-white text-2xl mr-4`}>
          {icon}
        </div>
        <div>
          <div className="text-sm text-gray-600 dark:text-gray-400">
            {title}
          </div>
          <div className="text-2xl font-bold text-gray-900 dark:text-white">
            {value}
          </div>
        </div>
      </div>
    </div>
  );
}
