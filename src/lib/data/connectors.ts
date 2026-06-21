export interface Provider {
  id: string;
  name: string;
  /** Sub-capabilities included in this single OAuth connection. */
  features: string[];
  color: string;
  textColor?: string;
  requiresOAuth?: boolean;
  /** Providers in the same group are mutually exclusive — connecting one auto-disconnects the others. */
  group?: string;
}

export const PROVIDERS: Provider[] = [
  {
    id: 'google',
    name: 'Google',
    features: ['Gmail', 'Calendar'],
    color: '#4285F4',
    textColor: '#ffffff',
    requiresOAuth: true,
    group: 'mail',
  },
  {
    id: 'outlook',
    name: 'Outlook',
    features: ['Mail', 'Calendar'],
    color: '#0078D4',
    textColor: '#ffffff',
    requiresOAuth: true,
    group: 'mail',
  },
];
