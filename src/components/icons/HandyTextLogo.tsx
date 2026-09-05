import React from "react";
const productName = "HandyFork";
const HandyTextLogo = ({
  width,
  height,
  className,
}: {
  width?: number;
  height?: number;
  className?: string;
}) => (
  <svg
    width={width}
    height={height ?? 40}
    className={className}
    viewBox="0 0 190 48"
    role="img"
    aria-label="HandyFork"
  >
    <rect x="2" y="4" width="40" height="40" rx="12" fill="#6255df" />
    <path
      d="M13 23v4m6-10v16m6-19v22m6-16v10"
      stroke="white"
      strokeWidth="3"
      strokeLinecap="round"
    />
    <text
      x="52"
      y="32"
      fill="currentColor"
      fontSize="25"
      fontWeight="600"
      fontFamily="sans-serif"
    >
      {productName}
    </text>
  </svg>
);
export default HandyTextLogo;
