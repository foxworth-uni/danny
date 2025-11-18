// This file demonstrates various CODE SMELLS that Danny can detect

// ============================================
// CODE SMELL 1: LONG FUNCTION (>50 lines)
// ============================================
export function processUserRegistration(
  email: string,
  password: string,
  firstName: string,
  lastName: string,
  age: number,
  address: string,
  city: string,
  state: string,
  zipCode: string
) {
  // Validate email
  if (!email || email.length === 0) {
    throw new Error('Email is required');
  }
  if (!email.includes('@')) {
    throw new Error('Invalid email format');
  }
  if (email.length > 255) {
    throw new Error('Email too long');
  }

  // Validate password
  if (!password || password.length === 0) {
    throw new Error('Password is required');
  }
  if (password.length < 8) {
    throw new Error('Password must be at least 8 characters');
  }
  if (!/[A-Z]/.test(password)) {
    throw new Error('Password must contain uppercase letter');
  }
  if (!/[a-z]/.test(password)) {
    throw new Error('Password must contain lowercase letter');
  }
  if (!/[0-9]/.test(password)) {
    throw new Error('Password must contain number');
  }

  // Validate names
  if (!firstName || firstName.length === 0) {
    throw new Error('First name is required');
  }
  if (!lastName || lastName.length === 0) {
    throw new Error('Last name is required');
  }
  if (firstName.length > 50) {
    throw new Error('First name too long');
  }
  if (lastName.length > 50) {
    throw new Error('Last name too long');
  }

  // Validate age
  if (!age) {
    throw new Error('Age is required');
  }
  if (age < 13) {
    throw new Error('Must be at least 13 years old');
  }
  if (age > 120) {
    throw new Error('Invalid age');
  }

  // Validate address
  if (!address || address.length === 0) {
    throw new Error('Address is required');
  }
  if (address.length > 200) {
    throw new Error('Address too long');
  }

  // Validate city
  if (!city || city.length === 0) {
    throw new Error('City is required');
  }

  // Validate state
  if (!state || state.length === 0) {
    throw new Error('State is required');
  }
  if (state.length !== 2) {
    throw new Error('State must be 2 characters');
  }

  // Validate zip code
  if (!zipCode || zipCode.length === 0) {
    throw new Error('Zip code is required');
  }
  if (!/^\d{5}(-\d{4})?$/.test(zipCode)) {
    throw new Error('Invalid zip code format');
  }

  // Create user object
  const user = {
    email: email.toLowerCase(),
    password: hashPassword(password),
    firstName,
    lastName,
    age,
    address,
    city,
    state: state.toUpperCase(),
    zipCode,
    createdAt: new Date(),
    isActive: true,
    emailVerified: false,
  };

  // Save to database
  console.log('Saving user:', user);

  return user;
}
// This function is 104 lines - should trigger LongFunction smell!

function hashPassword(password: string): string {
  return `hashed_${password}`;
}

// ============================================
// CODE SMELL 2: TOO MANY PARAMETERS (>4)
// ============================================
export function createProduct(
  name: string,
  description: string,
  price: number,
  category: string,
  sku: string,
  quantity: number,
  weight: number,
  dimensions: string
) {
  // 8 parameters - should trigger TooManyParameters smell!
  return {
    name,
    description,
    price,
    category,
    sku,
    quantity,
    weight,
    dimensions,
  };
}

export function sendNotification(
  userId: string,
  title: string,
  message: string,
  priority: string,
  channels: string[],
  scheduledAt: Date
) {
  // 6 parameters - should trigger TooManyParameters smell!
  console.log('Sending notification:', { userId, title, message, priority, channels, scheduledAt });
}

// ============================================
// CODE SMELL 3: LARGE CLASS (>300 lines)
// ============================================
export class UserManagementService {
  private users: Map<string, any> = new Map();
  private sessions: Map<string, any> = new Map();
  private permissions: Map<string, string[]> = new Map();

  // Method 1
  public createUser(userData: any) {
    const id = this.generateId();
    this.users.set(id, userData);
    return id;
  }

  // Method 2
  public getUser(id: string) {
    return this.users.get(id);
  }

  // Method 3
  public updateUser(id: string, updates: any) {
    const user = this.users.get(id);
    if (user) {
      this.users.set(id, { ...user, ...updates });
    }
  }

  // Method 4
  public deleteUser(id: string) {
    this.users.delete(id);
    this.sessions.delete(id);
    this.permissions.delete(id);
  }

  // Method 5
  public createSession(userId: string) {
    const sessionId = this.generateId();
    this.sessions.set(sessionId, { userId, createdAt: new Date() });
    return sessionId;
  }

  // Method 6
  public validateSession(sessionId: string) {
    return this.sessions.has(sessionId);
  }

  // Method 7
  public destroySession(sessionId: string) {
    this.sessions.delete(sessionId);
  }

  // Method 8
  public grantPermission(userId: string, permission: string) {
    const perms = this.permissions.get(userId) || [];
    perms.push(permission);
    this.permissions.set(userId, perms);
  }

  // Method 9
  public revokePermission(userId: string, permission: string) {
    const perms = this.permissions.get(userId) || [];
    this.permissions.set(userId, perms.filter(p => p !== permission));
  }

  // Method 10
  public hasPermission(userId: string, permission: string) {
    const perms = this.permissions.get(userId) || [];
    return perms.includes(permission);
  }

  // Method 11
  public getAllUsers() {
    return Array.from(this.users.values());
  }

  // Method 12
  public searchUsers(query: string) {
    return this.getAllUsers().filter((user: any) =>
      user.email?.includes(query) || user.name?.includes(query)
    );
  }

  // Method 13
  public getUserCount() {
    return this.users.size;
  }

  // Method 14
  public getActiveSessionCount() {
    return this.sessions.size;
  }

  // Method 15
  public cleanupExpiredSessions() {
    const now = Date.now();
    const maxAge = 24 * 60 * 60 * 1000; // 24 hours

    for (const [sessionId, session] of this.sessions.entries()) {
      if (now - session.createdAt.getTime() > maxAge) {
        this.sessions.delete(sessionId);
      }
    }
  }

  // Method 16
  public exportUsers() {
    return JSON.stringify(Array.from(this.users.entries()));
  }

  // Method 17
  public importUsers(data: string) {
    const entries = JSON.parse(data);
    this.users = new Map(entries);
  }

  // Method 18
  public backupData() {
    return {
      users: Array.from(this.users.entries()),
      sessions: Array.from(this.sessions.entries()),
      permissions: Array.from(this.permissions.entries()),
    };
  }

  // Method 19
  public restoreData(backup: any) {
    this.users = new Map(backup.users);
    this.sessions = new Map(backup.sessions);
    this.permissions = new Map(backup.permissions);
  }

  // Method 20
  public clearAllData() {
    this.users.clear();
    this.sessions.clear();
    this.permissions.clear();
  }

  // Helper method
  private generateId(): string {
    return Math.random().toString(36).substring(2, 15);
  }

  // More methods to make this class even larger...

  public getUsersByRole(role: string) {
    return this.getAllUsers().filter((user: any) => user.role === role);
  }

  public updateUserRole(userId: string, newRole: string) {
    const user = this.users.get(userId);
    if (user) {
      user.role = newRole;
      this.users.set(userId, user);
    }
  }

  public disableUser(userId: string) {
    const user = this.users.get(userId);
    if (user) {
      user.isActive = false;
      this.users.set(userId, user);
      this.destroyAllUserSessions(userId);
    }
  }

  public enableUser(userId: string) {
    const user = this.users.get(userId);
    if (user) {
      user.isActive = true;
      this.users.set(userId, user);
    }
  }

  public destroyAllUserSessions(userId: string) {
    for (const [sessionId, session] of this.sessions.entries()) {
      if (session.userId === userId) {
        this.sessions.delete(sessionId);
      }
    }
  }

  public getUserSessions(userId: string) {
    const sessions = [];
    for (const [sessionId, session] of this.sessions.entries()) {
      if (session.userId === userId) {
        sessions.push({ sessionId, ...session });
      }
    }
    return sessions;
  }

  public validateUserCredentials(email: string, password: string) {
    for (const user of this.users.values()) {
      if (user.email === email && user.password === password) {
        return user;
      }
    }
    return null;
  }

  public resetUserPassword(userId: string, newPassword: string) {
    const user = this.users.get(userId);
    if (user) {
      user.password = hashPassword(newPassword);
      this.users.set(userId, user);
      this.destroyAllUserSessions(userId);
    }
  }

  public sendPasswordResetEmail(email: string) {
    const user = this.findUserByEmail(email);
    if (user) {
      console.log(`Sending password reset email to ${email}`);
      return true;
    }
    return false;
  }

  private findUserByEmail(email: string) {
    for (const user of this.users.values()) {
      if (user.email === email) {
        return user;
      }
    }
    return null;
  }
}
// This class is over 300 lines with 20+ methods - should trigger LargeClass smell!

// ============================================
// GOOD CODE (for comparison)
// ============================================
export function calculateTotal(items: Array<{ price: number; quantity: number }>) {
  return items.reduce((sum, item) => sum + (item.price * item.quantity), 0);
}

export function formatDate(date: Date): string {
  return date.toISOString().split('T')[0];
}

// Small, focused class with single responsibility
export class Logger {
  log(message: string) {
    console.log(`[LOG] ${message}`);
  }

  error(message: string) {
    console.error(`[ERROR] ${message}`);
  }
}
