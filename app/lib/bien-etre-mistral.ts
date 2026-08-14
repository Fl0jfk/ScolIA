const MISTRAL_URL = "https://api.mistral.ai/v1/chat/completions";

function sleep(ms: number) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

async function fetchMistralWithRetry(
  body: unknown,
  apiKey: string,
  attempts = 3,
): Promise<Response> {
  let lastResponse: Response | null = null;
  for (let i = 0; i < attempts; i += 1) {
    const res = await fetch(MISTRAL_URL, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        Authorization: `Bearer ${apiKey}`,
      },
      body: JSON.stringify(body),
    });
    if (res.ok) return res;
    lastResponse = res;
    if (![429, 500, 502, 503, 504].includes(res.status) || i === attempts - 1) {
      return res;
    }
    await sleep(350 * (i + 1));
  }
  return lastResponse!;
}

export async function mistralChatText(
  apiKey: string,
  messages: Array<{ role: "system" | "user" | "assistant"; content: string }>,
  temperature = 0.4,
): Promise<string | null> {
  const res = await fetchMistralWithRetry(
    {
      model: "mistral-small-latest",
      temperature,
      messages,
    },
    apiKey,
  );
  if (!res.ok) return null;
  const data = await res.json();
  const text = data?.choices?.[0]?.message?.content;
  return typeof text === "string" ? text.trim() : null;
}

export async function mistralJsonObject<T>(
  apiKey: string,
  prompt: string,
): Promise<T | null> {
  const res = await fetchMistralWithRetry(
    {
      model: "mistral-small-latest",
      temperature: 0,
      response_format: { type: "json_object" },
      messages: [{ role: "user", content: prompt }],
    },
    apiKey,
  );
  if (!res.ok) return null;
  const data = await res.json();
  try {
    return JSON.parse(data?.choices?.[0]?.message?.content ?? "{}") as T;
  } catch {
    return null;
  }
}
