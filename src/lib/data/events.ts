export type EventType = 'flight' | 'hotel' | 'meeting' | 'restaurant' | 'taxi' | 'train';

export interface TimelineEvent {
  id: string;
  type: EventType;
  title: string;
  subtitle?: string;
  location?: string;
  start: Date;
  end?: Date;
}

const TYPE_META: Record<EventType, { label: string; color: string }> = {
  flight: { label: 'Flight', color: '#4f8ef7' },
  hotel: { label: 'Hotel', color: '#7c5cfc' },
  meeting: { label: 'Meeting', color: '#34d399' },
  restaurant: { label: 'Restaurant', color: '#f59e0b' },
  taxi: { label: 'Taxi', color: '#fbbf24' },
  train: { label: 'Train', color: '#06b6d4' },
};

export function typeMeta(type: EventType) {
  return TYPE_META[type];
}

/** Generate mock events anchored around today so the timeline always feels current. */
export function getMockEvents(): TimelineEvent[] {
  const now = new Date();
  const at = (dayOffset: number, h: number, m = 0): Date => {
    const d = new Date(now);
    d.setDate(d.getDate() + dayOffset);
    d.setHours(h, m, 0, 0);
    return d;
  };

  return [
    // ── Past ──
    { id: '1', type: 'flight', title: 'LHR → CDG', subtitle: 'BA 308 · Seat 4A', start: at(-3, 7, 30) },
    { id: '2', type: 'hotel', title: 'Le Meurice — check-out', location: 'Paris', start: at(-3, 11, 0) },
    {
      id: '3',
      type: 'meeting',
      title: 'Client review · Vega Capital',
      location: 'Paris',
      start: at(-3, 14, 0),
      end: at(-3, 15, 30),
    },
    { id: '4', type: 'restaurant', title: 'Dinner · Septime', subtitle: 'Party of 3', start: at(-3, 20, 0) },

    // ── Today ──
    {
      id: '5',
      type: 'meeting',
      title: 'Weekly leadership sync',
      start: at(0, 9, 0),
      end: at(0, 10, 0),
    },
    { id: '6', type: 'meeting', title: '1:1 with Sarah', start: at(0, 11, 0), end: at(0, 11, 30) },
    { id: '7', type: 'taxi', title: 'Taxi to Heathrow T5', subtitle: 'Uber · pickup 18:40', start: at(0, 18, 40) },
    {
      id: '8',
      type: 'flight',
      title: 'LHR → JFK',
      subtitle: 'BA 175 · Seat 3A · boarding 20:15',
      start: at(0, 21, 15),
    },

    // ── Tomorrow ──
    { id: '9', type: 'hotel', title: 'The Pierre — check-in', location: 'New York', start: at(1, 9, 0) },
    {
      id: '10',
      type: 'meeting',
      title: 'Pitch · Arrow Ventures',
      location: '5th Ave, New York',
      start: at(1, 14, 0),
      end: at(1, 16, 0),
    },
    {
      id: '11',
      type: 'restaurant',
      title: 'Dinner · Le Bernardin',
      subtitle: '8:00 PM · party of 4',
      start: at(1, 20, 0),
    },

    // ── Later ──
    { id: '12', type: 'flight', title: 'JFK → LHR', subtitle: 'BA 178', start: at(4, 18, 30) },
    {
      id: '13',
      type: 'meeting',
      title: 'Q3 board meeting',
      location: 'London HQ',
      start: at(7, 10, 0),
      end: at(7, 12, 0),
    },
  ];
}
