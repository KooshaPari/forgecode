import React from 'react';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { AuthProvider, useAuth } from '@/contexts/AuthContext';
import { supabase } from '@/lib/supabase';

// Mock the supabase client
jest.mock('@/lib/supabase', () => ({
  supabase: {
    auth: {
      getSession: jest.fn().mockResolvedValue({ data: { session: null } }),
      onAuthStateChange: jest.fn().mockReturnValue({
        data: { subscription: { unsubscribe: jest.fn() } },
      }),
      signInWithPassword: jest.fn(),
      signUp: jest.fn(),
      signOut: jest.fn(),
      resetPasswordForEmail: jest.fn(),
    },
  },
  signIn: jest.fn(),
  signUp: jest.fn(),
  signOut: jest.fn(),
  resetPassword: jest.fn(),
}));

// Test component that uses the auth context
const TestComponent = () => {
  const { user, signIn, signUp, signOut, resetPassword } = useAuth();
  
  return (
    <div>
      <div data-testid="user-status">{user ? 'Logged In' : 'Logged Out'}</div>
      <button 
        data-testid="sign-in-button" 
        onClick={() => signIn('test@example.com', 'password')}
      >
        Sign In
      </button>
      <button 
        data-testid="sign-up-button" 
        onClick={() => signUp('test@example.com', 'password')}
      >
        Sign Up
      </button>
      <button 
        data-testid="sign-out-button" 
        onClick={() => signOut()}
      >
        Sign Out
      </button>
      <button 
        data-testid="reset-password-button" 
        onClick={() => resetPassword('test@example.com')}
      >
        Reset Password
      </button>
    </div>
  );
};

describe('AuthContext', () => {
  beforeEach(() => {
    jest.clearAllMocks();
  });

  it('provides authentication context to children', () => {
    render(
      <AuthProvider>
        <TestComponent />
      </AuthProvider>
    );

    expect(screen.getByTestId('user-status')).toHaveTextContent('Logged Out');
  });

  it('calls signIn with correct parameters', async () => {
    supabase.auth.signInWithPassword.mockResolvedValueOnce({
      data: { user: { id: '123' }, session: { user: { id: '123' } } },
      error: null,
    });

    render(
      <AuthProvider>
        <TestComponent />
      </AuthProvider>
    );

    fireEvent.click(screen.getByTestId('sign-in-button'));

    await waitFor(() => {
      expect(supabase.auth.signInWithPassword).toHaveBeenCalledWith({
        email: 'test@example.com',
        password: 'password',
      });
    });
  });

  it('calls signUp with correct parameters', async () => {
    supabase.auth.signUp.mockResolvedValueOnce({
      data: { user: { id: '123' }, session: null },
      error: null,
    });

    render(
      <AuthProvider>
        <TestComponent />
      </AuthProvider>
    );

    fireEvent.click(screen.getByTestId('sign-up-button'));

    await waitFor(() => {
      expect(supabase.auth.signUp).toHaveBeenCalledWith({
        email: 'test@example.com',
        password: 'password',
      });
    });
  });

  it('calls signOut when sign out button is clicked', async () => {
    supabase.auth.signOut.mockResolvedValueOnce({ error: null });

    render(
      <AuthProvider>
        <TestComponent />
      </AuthProvider>
    );

    fireEvent.click(screen.getByTestId('sign-out-button'));

    await waitFor(() => {
      expect(supabase.auth.signOut).toHaveBeenCalled();
    });
  });

  it('calls resetPassword with correct parameters', async () => {
    supabase.auth.resetPasswordForEmail.mockResolvedValueOnce({
      data: {},
      error: null,
    });

    // Mock window.location.origin
    Object.defineProperty(window, 'location', {
      value: {
        origin: 'http://localhost:3000',
      },
      writable: true,
    });

    render(
      <AuthProvider>
        <TestComponent />
      </AuthProvider>
    );

    fireEvent.click(screen.getByTestId('reset-password-button'));

    await waitFor(() => {
      expect(supabase.auth.resetPasswordForEmail).toHaveBeenCalledWith(
        'test@example.com',
        { redirectTo: 'http://localhost:3000/reset-password' }
      );
    });
  });

  it('updates user state when session changes', async () => {
    // First render with no user
    supabase.auth.getSession.mockResolvedValueOnce({ 
      data: { session: null } 
    });

    const { rerender } = render(
      <AuthProvider>
        <TestComponent />
      </AuthProvider>
    );

    expect(screen.getByTestId('user-status')).toHaveTextContent('Logged Out');

    // Simulate auth state change
    const mockAuthChange = supabase.auth.onAuthStateChange.mock.calls[0][1];
    mockAuthChange('SIGNED_IN', { 
      user: { id: '123' } 
    });

    await waitFor(() => {
      expect(screen.getByTestId('user-status')).toHaveTextContent('Logged In');
    });
  });
});
