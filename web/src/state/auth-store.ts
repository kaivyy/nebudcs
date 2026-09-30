/**
 * NebuDCS Client Authentication & RBAC Store.
 *
 * Manages active user role, authentication tokens, and credentials persistence.
 * Maps cleanly to production RBAC tokens defined in nebudcs.env:
 * - Admin: Full access (Studio authoring, simulation host control, admin endpoints)
 * - Instructor: Session orchestration, scenario management, fault injection, grading
 * - Operator: Process monitoring, faceplate tuning, alarm acknowledge, simulation play/pause
 * - Trainee: Supervised operation only; administrative actions rejected server-side
 */

export type UserRoleName = "admin" | "engineer" | "instructor" | "operator" | "trainee" | "custom";

export interface RoleDefinition {
  id: UserRoleName;
  name: string;
  badge: string;
  badgeClass: string;
  defaultToken: string;
  description: string;
}

export const PRESET_ROLES: Record<Exclude<UserRoleName, "custom">, RoleDefinition> = {
  admin: {
    id: "admin",
    name: "Admin / Lead Architect",
    badge: "ADMIN",
    badgeClass: "badge-alarm",
    defaultToken: "sec-admin-change-me-in-production-482a9f",
    description: "Full administrative, enterprise cluster, and simulation host authority",
  },
  engineer: {
    id: "engineer",
    name: "Control & Process Engineer",
    badge: "ENGINEER",
    badgeClass: "badge-warn",
    defaultToken: "sec-eng-change-me-in-production-d53e7a",
    description: "Engineering design, P&ID topology, control algorithms, validation",
  },
  instructor: {
    id: "instructor",
    name: "Training Instructor",
    badge: "INSTRUCTOR",
    badgeClass: "badge-warn",
    defaultToken: "sec-inst-change-me-in-production-b71c3e",
    description: "Scenario authoring, session orchestration, fault injection, and grading",
  },
  operator: {
    id: "operator",
    name: "Console Operator",
    badge: "OPERATOR",
    badgeClass: "badge-info",
    defaultToken: "sec-oper-change-me-in-production-c92d4f",
    description: "Process supervision, controller tuning, alarm acknowledgement",
  },
  trainee: {
    id: "trainee",
    name: "Trainee Operator",
    badge: "TRAINEE",
    badgeClass: "badge-good",
    defaultToken: "sec-train-change-me-in-production-e10a8b",
    description: "Restricted operating station with server-side administrative guards",
  },
};

export type AuthListener = (role: UserRoleName, token: string) => void;

class AuthStore {
  private currentRole: UserRoleName = "admin";
  private currentToken: string = PRESET_ROLES.admin.defaultToken;
  private listeners: Set<AuthListener> = new Set();

  constructor() {
    this.initFromStorage();
  }

  private initFromStorage() {
    if (typeof window === "undefined" || !window.localStorage) {
      return;
    }

    // 1. Check URL query params for ?token=... or ?role=...
    try {
      const params = new URLSearchParams(window.location.search);
      const qToken = params.get("token");
      const qRole = params.get("role") as UserRoleName;

      if (qToken) {
        this.currentToken = qToken;
        this.currentRole = qRole || "custom";
        this.saveToStorage();
        return;
      }

      if (qRole && qRole in PRESET_ROLES) {
        this.currentRole = qRole;
        this.currentToken = PRESET_ROLES[qRole as keyof typeof PRESET_ROLES].defaultToken;
        this.saveToStorage();
        return;
      }
    } catch {
      // Ignore URL parsing errors in non-browser environments
    }

    // 2. Check localStorage
    const savedToken = localStorage.getItem("nebudcs_auth_token");
    const savedRole = localStorage.getItem("nebudcs_auth_role") as UserRoleName;

    if (savedToken) {
      this.currentToken = savedToken;
      this.currentRole = savedRole || "admin";
    } else {
      // Default out of the box to Admin so all initial flows work seamlessly
      this.currentRole = "admin";
      this.currentToken = PRESET_ROLES.admin.defaultToken;
      this.saveToStorage();
    }
  }

  private saveToStorage() {
    if (typeof window !== "undefined" && window.localStorage) {
      localStorage.setItem("nebudcs_auth_token", this.currentToken);
      localStorage.setItem("nebudcs_auth_role", this.currentRole);
    }
  }

  public getRole(): UserRoleName {
    return this.currentRole;
  }

  public getToken(): string {
    return this.currentToken;
  }

  public getRoleDefinition(): RoleDefinition {
    if (this.currentRole in PRESET_ROLES) {
      return PRESET_ROLES[this.currentRole as keyof typeof PRESET_ROLES];
    }
    return {
      id: "custom",
      name: "Custom Token",
      badge: "CUSTOM",
      badgeClass: "badge-info",
      defaultToken: this.currentToken,
      description: "User-defined authentication secret",
    };
  }

  public setRole(role: UserRoleName, customToken?: string) {
    this.currentRole = role;
    if (role === "custom") {
      this.currentToken = customToken || this.currentToken;
    } else if (role in PRESET_ROLES) {
      this.currentToken = PRESET_ROLES[role as keyof typeof PRESET_ROLES].defaultToken;
    }
    this.saveToStorage();
    this.notify();
  }

  public setToken(token: string) {
    this.currentToken = token;
    // Check if token matches any known preset
    let found = false;
    for (const [r, def] of Object.entries(PRESET_ROLES)) {
      if (def.defaultToken === token) {
        this.currentRole = r as UserRoleName;
        found = true;
        break;
      }
    }
    if (!found) {
      this.currentRole = "custom";
    }
    this.saveToStorage();
    this.notify();
  }

  public subscribe(listener: AuthListener): () => void {
    this.listeners.add(listener);
    listener(this.currentRole, this.currentToken);
    return () => this.listeners.delete(listener);
  }

  private notify() {
    for (const listener of this.listeners) {
      listener(this.currentRole, this.currentToken);
    }
  }
}

export const authStore = new AuthStore();
