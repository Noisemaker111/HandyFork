const HandyHand = ({
  width = 24,
  height = 24,
}: {
  width?: number | string;
  height?: number | string;
}) => (
  <svg
    width={width}
    height={height}
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    strokeWidth="2"
    strokeLinecap="round"
    aria-hidden="true"
  >
    <path d="M4 10v4m4-7v10m4-14v18m4-14v10m4-7v4" />
  </svg>
);
export default HandyHand;
