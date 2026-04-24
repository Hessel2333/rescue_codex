import { useEffect, useRef } from "react";
import clsx from "clsx";
import type { EChartsOption } from "echarts";
import * as echarts from "echarts/core";
import { BarChart, HeatmapChart, LineChart, PieChart, ScatterChart } from "echarts/charts";
import {
  CalendarComponent,
  GraphicComponent,
  GridComponent,
  LegendComponent,
  TooltipComponent,
  VisualMapComponent,
} from "echarts/components";
import { CanvasRenderer } from "echarts/renderers";

echarts.use([
  BarChart,
  HeatmapChart,
  LineChart,
  PieChart,
  ScatterChart,
  CalendarComponent,
  GridComponent,
  TooltipComponent,
  LegendComponent,
  GraphicComponent,
  VisualMapComponent,
  CanvasRenderer,
]);

type EChartProps = {
  option: EChartsOption;
  height?: number;
  className?: string;
};

export function EChart({ option, height = 300, className }: EChartProps) {
  const rootRef = useRef<HTMLDivElement | null>(null);
  const chartRef = useRef<echarts.EChartsType | null>(null);

  useEffect(() => {
    if (!rootRef.current) {
      return undefined;
    }

    const chart = echarts.init(rootRef.current, undefined, {
      renderer: "canvas",
    });
    chartRef.current = chart;

    const observer =
      typeof ResizeObserver === "undefined"
        ? null
        : new ResizeObserver(() => {
            chart.resize();
          });

    observer?.observe(rootRef.current);

    const handleWindowResize = () => {
      chart.resize();
    };
    window.addEventListener("resize", handleWindowResize);

    return () => {
      observer?.disconnect();
      window.removeEventListener("resize", handleWindowResize);
      chart.dispose();
      chartRef.current = null;
    };
  }, []);

  useEffect(() => {
    chartRef.current?.setOption(option, true);
  }, [option]);

  return <div ref={rootRef} className={clsx("w-full", className)} style={{ height }} />;
}
