import useSWR from "swr";
import { useRouter } from "next/navigation";

interface User {
  id: string;
  name: string;
  email: string;
  role: "admin" | "customer";
}

const fetcher = async (url: string) => {
  const res = await fetch(url);
  if (!res.ok) throw new Error("Unauthorized");
  const data = await res.json();
  return data.user as User;
};

export function useCurrentUser() {
  return useSWR<User>("/api/auth/me", fetcher, {
    revalidateOnFocus: false,
    shouldRetryOnError: false,
  });
}

export function useLogout() {
  const router = useRouter();

  return async () => {
    const res = await fetch("/api/auth/logout", { method: "POST" });
    const data = await res.json().catch(() => ({}));

    if (data.redirectUrl) {
      // Keycloak SSO logout — full page redirect to end-session endpoint
      window.location.href = data.redirectUrl;
    } else {
      router.push("/login");
    }
  };
}
