import { describe, expect, it } from "vitest";
import { humanErrorFromJobs } from "$lib/vnext/presentation/errors";

describe("humanErrorFromJobs", () => {
  it("uses message from errorJson", () => {
    const msg = humanErrorFromJobs(
      [
        {
          status: "failed",
          errorJson: JSON.stringify({ message: "El video no se pudo leer", code: "E1" }),
        },
      ],
      "fallback",
    );
    expect(msg).toBe("El video no se pudo leer");
  });

  it("falls back when no failed job", () => {
    expect(humanErrorFromJobs([{ status: "completed" }], "nada")).toBe("nada");
  });
});
