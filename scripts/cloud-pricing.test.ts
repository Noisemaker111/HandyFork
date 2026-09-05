import { test, expect } from "bun:test";
import {
  formatHourlyPrice,
  type CloudModel,
} from "../src/lib/cloudTranscription";

const model: CloudModel = {
  id: "test/stt",
  name: "Test",
  description: "",
  hourly_min: 0.1,
  hourly_max: 0.1,
  price_estimated: false,
  native_streaming: false,
};
test("hourly pricing displays precise low-cost rates and provider ranges", () => {
  expect(formatHourlyPrice(model, "en")).toBe("$0.10");
  expect(
    formatHourlyPrice(
      { ...model, hourly_min: 0.011988, hourly_max: 0.04 },
      "en",
    ),
  ).toBe("$0.012–$0.04");
});
test("missing or invalid prices never look free", () => {
  expect(formatHourlyPrice({ ...model, hourly_min: null }, "en")).toBeNull();
  expect(formatHourlyPrice({ ...model, hourly_min: NaN }, "en")).toBeNull();
});
