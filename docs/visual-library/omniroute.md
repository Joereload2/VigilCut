# OmniRoute image generation

## Role

OmniRoute is an **optional local gateway** (`localhost:20128/v1`) used only for **missing** library images after match fails.

VigilCut never requires OmniRoute to edit video, match library assets, or export.

## Config

See `.env.example`:

| Variable | Default |
|----------|---------|
| `OMNIROUTE_BASE_URL` | unset → mock provider |
| `OMNIROUTE_API_KEY` | optional |
| `OMNIROUTE_IMAGE_MODEL` | explicit model id (no `auto`) |
| `OMNIROUTE_FREE_TIER` | `true` |
| `VIGILCUT_PAID_PROVIDERS` | `0` |
| `VIGILCUT_IMAGE_PROVIDER` | `mock` to force offline |

## Contract

`POST {base}/images/generations` (OpenAI-compatible):

```json
{
  "model": "<explicit model>",
  "prompt": "...",
  "n": 1,
  "size": "1280x720",
  "response_format": "url"
}
```

Supports `data[].url` or `data[].b64_json`. URLs are downloaded immediately (size cap 25 MB). Magic-byte MIME validation + decode.

## Probe

Command / CLI: `visual_probe_image_provider` lists `/v1/models` and records `provider_capabilities`. Image routing via OmniRoute `auto` is **not** enabled until manual validation.

## Cost order

1. Reuse library  
2. Free mock / free OmniRoute  
3. Paid only if `VIGILCUT_PAID_PROVIDERS=1` and budget > 0  

Opportunistic generation only when `VIGILCUT_OPPORTUNISTIC=1` and free quota is meaningful — never generate solely to drain quota.
