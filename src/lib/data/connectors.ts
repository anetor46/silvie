export interface Provider {
  id: string;
  name: string;
  color: string;
  textColor?: string;
}

export interface ConnectorCategory {
  id: string;
  label: string;
  providers: Provider[];
}

export const CATEGORIES: ConnectorCategory[] = [
  {
    id: 'email',
    label: 'Email',
    providers: [
      { id: 'gmail', name: 'Gmail', color: '#EA4335', textColor: '#ffffff' },
      { id: 'outlook-mail', name: 'Outlook', color: '#0078D4', textColor: '#ffffff' },
    ],
  },
  {
    id: 'calendar',
    label: 'Calendar',
    providers: [
      { id: 'google-calendar', name: 'Google Calendar', color: '#4285F4', textColor: '#ffffff' },
      { id: 'outlook-calendar', name: 'Outlook Calendar', color: '#0078D4', textColor: '#ffffff' },
    ],
  },
];
