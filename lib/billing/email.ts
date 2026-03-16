import { Resend } from "resend";
import { appConfig } from "@/lib/app-config";

const RESEND_API_KEY = process.env.RESEND_API_KEY;
const FROM_EMAIL = process.env.BILLING_FROM_EMAIL ?? appConfig.billingEmail;

export const isEmailEnabled = !!RESEND_API_KEY;
const resend = isEmailEnabled ? new Resend(RESEND_API_KEY) : null;

export async function sendBillingEmail(params: {
  to: string;
  subject: string;
  html: string;
}): Promise<boolean> {
  if (!resend) {
    console.log(`[Email Stub] To: ${params.to} | Subject: ${params.subject}`);
    return false;
  }

  try {
    await resend.emails.send({
      from: FROM_EMAIL,
      to: params.to,
      subject: params.subject,
      html: params.html,
    });
    return true;
  } catch (err) {
    console.error("Email send failed:", err);
    return false;
  }
}
