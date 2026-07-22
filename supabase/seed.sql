-- Minimal dev seed (run after migration). Does not insert auth.users.
-- Safe for local Supabase only.

INSERT INTO public.themes (id, slug, title, description)
VALUES (
  '11111111-1111-1111-1111-111111111111',
  'economia-dinero-negocios',
  'Economía, dinero y negocios',
  'Seed theme for development'
) ON CONFLICT DO NOTHING;

INSERT INTO public.visual_concepts (
  id, theme_id, canonical_key, title, literal_description, meanings,
  positive_contexts, negative_contexts, hard_exclusions, status, priority
) VALUES (
  '22222222-2222-2222-2222-222222222221',
  '11111111-1111-1111-1111-111111111111',
  'persona_comparando_precios_en_supermercado',
  'Persona comparando precios en supermercado',
  '["persona","supermercado","etiquetas de precios"]'::jsonb,
  '["inflación","costo de vida"]'::jsonb,
  '["economía doméstica","presupuesto familiar"]'::jsonb,
  '["criptomonedas","lujo"]'::jsonb,
  '["marcas comerciales","billetes flotando"]'::jsonb,
  'priority',
  80
) ON CONFLICT DO NOTHING;
