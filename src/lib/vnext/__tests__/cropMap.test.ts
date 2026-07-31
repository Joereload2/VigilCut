import { describe, expect, it } from "vitest";
import {
  clampSection,
  physicalAspectOfBox,
  regionToPanelStageStyle,
  regionToPanelVideoStyle,
  sectionBox,
} from "$lib/vnext/types/crop";

describe("flexible frames", () => {
  it("allows free aspect (no lock)", () => {
    const wide = clampSection({
      id: "w",
      centerX: 0.5,
      centerY: 0.5,
      widthFrac: 0.8,
      heightFrac: 0.3,
    });
    expect(wide.widthFrac).toBeCloseTo(0.8, 5);
    expect(wide.heightFrac).toBeCloseTo(0.3, 5);
    // Physical AR wider than 9:16
    expect(physicalAspectOfBox(wide)).toBeGreaterThan(9 / 16);
  });

  it("allows tall free selection", () => {
    const tall = clampSection({
      id: "t",
      centerX: 0.4,
      centerY: 0.5,
      widthFrac: 0.2,
      heightFrac: 0.95,
    });
    expect(tall.widthFrac).toBeCloseTo(0.2, 5);
    expect(tall.heightFrac).toBeCloseTo(0.95, 5);
  });
});

describe("regionToPanelStageStyle", () => {
  it("maps selection to fill fit-box", () => {
    const r = clampSection({
      id: "a",
      centerX: 0.5,
      centerY: 0.5,
      widthFrac: 0.3,
      heightFrac: 0.9,
    });
    const style = regionToPanelStageStyle(r);
    expect(style).toMatch(/width:/);
    expect(style).toMatch(/height:/);
  });

  it("legacy video style includes cover", () => {
    const r = clampSection({
      id: "x",
      centerX: 0.5,
      centerY: 0.5,
      widthFrac: 0.3,
      heightFrac: 0.9,
    });
    expect(regionToPanelVideoStyle(r)).toMatch(/object-fit:cover/);
  });

  it("sectionBox matches region geometry", () => {
    const r = clampSection({
      id: "c",
      centerX: 0.6,
      centerY: 0.4,
      widthFrac: 0.3,
      heightFrac: 0.5,
    });
    const box = sectionBox(r);
    expect(box.left).toBeCloseTo((r.centerX - r.widthFrac / 2) * 100, 5);
    expect(box.top).toBeCloseTo((r.centerY - r.heightFrac / 2) * 100, 5);
    expect(box.w).toBeCloseTo(r.widthFrac * 100, 5);
    expect(box.h).toBeCloseTo(r.heightFrac * 100, 5);
  });
});
