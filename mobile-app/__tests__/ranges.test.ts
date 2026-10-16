import { downsample, hrAxis, RANGES, splitOnGaps } from "../src/features/history/ranges";

describe("history ranges", () => {
  it("asks the backend for 1m, 1h and 1d rollups", () => {
    expect([RANGES.day.resolution, RANGES.week.resolution, RANGES.month.resolution]).toEqual(["1m", "1h", "1d"]);
  });

  it("averages minute buckets into display bins", () => {
    const bin = 600_000;
    const pts = [
      { t: 0, hr: 60 },
      { t: 60_000, hr: 80 },
      { t: bin, hr: 100 },
    ];
    expect(downsample(pts, bin)).toEqual([
      { t: 0, hr: 70 },
      { t: bin, hr: 100 },
    ]);
  });

  it("breaks the line on holes", () => {
    const runs = splitOnGaps([{ t: 0, hr: 1 }, { t: 1, hr: 1 }, { t: 10, hr: 1 }], 3);
    expect(runs.map((r) => r.length)).toEqual([2, 1]);
  });

  it("keeps the axis on steps of 20 bpm and at least 60-140", () => {
    expect(hrAxis([])).toEqual({ min: 60, max: 140 });
    expect(hrAxis([62, 154])).toEqual({ min: 60, max: 160 });
    expect(hrAxis([45, 90])).toEqual({ min: 40, max: 120 });
  });
});
