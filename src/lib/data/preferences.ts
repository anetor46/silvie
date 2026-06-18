import type { Provider } from './connectors';

export interface PreferenceCategory {
  id: string;
  label: string;
  description: string;
  providers: Provider[];
}

export const PREFERENCE_CATEGORIES: PreferenceCategory[] = [
  {
    id: 'taxi',
    label: 'Taxis',
    description: 'Used when Silvie suggests a ride to or from a venue.',
    providers: [
      { id: 'uber', name: 'Uber', color: '#000000', textColor: '#ffffff' },
      { id: 'bolt', name: 'Bolt', color: '#34D186', textColor: '#ffffff' },
    ],
  },
  {
    id: 'travel',
    label: 'Hotels & Flights',
    description: 'Used when Silvie surfaces a flight or hotel booking link.',
    providers: [
      { id: 'booking', name: 'Booking.com', color: '#003580', textColor: '#ffffff' },
      { id: 'expedia', name: 'Expedia', color: '#FFC72C', textColor: '#1a1a1a' },
    ],
  },
  {
    id: 'restaurant',
    label: 'Restaurants',
    description: 'Used when Silvie suggests reserving a table.',
    providers: [{ id: 'opentable', name: 'OpenTable', color: '#DA3743', textColor: '#ffffff' }],
  },
  {
    id: 'food-delivery',
    label: 'Food delivery',
    description: 'Used when Silvie suggests ordering food in.',
    providers: [
      { id: 'uber-eats', name: 'Uber Eats', color: '#06C167', textColor: '#ffffff' },
      { id: 'deliveroo', name: 'Deliveroo', color: '#00CCBC', textColor: '#ffffff' },
    ],
  },
];
