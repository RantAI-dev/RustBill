import { z } from "zod";

export const createApiKeySchema = z.object({
  name: z.string().min(1, "Name is required").max(255),
});

export type CreateApiKey = z.infer<typeof createApiKeySchema>;
