// UNUSED FILE - Analytics utilities that are never actually used

export function trackPageView(page) {
  if (typeof window !== 'undefined') {
    console.log('Page view:', page);
  }
}

export function trackEvent(eventName, data) {
  if (typeof window !== 'undefined') {
    console.log('Event:', eventName, data);
  }
}

export function initAnalytics(apiKey) {
  console.log('Analytics initialized with key:', apiKey);
}

// This entire file is never imported anywhere in the app
// All exports are unused
