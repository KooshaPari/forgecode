import React from 'react';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import ForgotPasswordForm from '@/components/auth/ForgotPasswordForm';
import { useAuth } from '@/contexts/AuthContext';

// Mock the useAuth hook
jest.mock('@/contexts/AuthContext', () => ({
  useAuth: jest.fn(),
}));

describe('ForgotPasswordForm', () => {
  const mockResetPassword = jest.fn();
  
  beforeEach(() => {
    jest.clearAllMocks();
    
    // Setup mocks
    (useAuth as jest.Mock).mockReturnValue({
      resetPassword: mockResetPassword,
    });
  });
  
  it('renders the forgot password form correctly', () => {
    render(<ForgotPasswordForm />);
    
    expect(screen.getByText('Reset Your Password')).toBeInTheDocument();
    expect(screen.getByText(/enter your email address/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/email address/i)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /send reset link/i })).toBeInTheDocument();
    expect(screen.getByText(/back to sign in/i)).toBeInTheDocument();
  });
  
  it('validates email input', async () => {
    render(<ForgotPasswordForm />);
    
    // Try to submit with empty email
    fireEvent.click(screen.getByRole('button', { name: /send reset link/i }));
    
    await waitFor(() => {
      expect(screen.getByText(/please enter your email address/i)).toBeInTheDocument();
    });
    
    // Verify resetPassword was not called
    expect(mockResetPassword).not.toHaveBeenCalled();
  });
  
  it('handles successful password reset request', async () => {
    mockResetPassword.mockResolvedValueOnce({ 
      data: {},
      error: null 
    });
    
    render(<ForgotPasswordForm />);
    
    // Fill in the email
    fireEvent.change(screen.getByLabelText(/email address/i), {
      target: { value: 'test@example.com' },
    });
    
    // Submit the form
    fireEvent.click(screen.getByRole('button', { name: /send reset link/i }));
    
    await waitFor(() => {
      // Verify resetPassword was called with correct parameters
      expect(mockResetPassword).toHaveBeenCalledWith('test@example.com');
      
      // Verify success message is displayed
      expect(screen.getByText(/check your email for a password reset link/i)).toBeInTheDocument();
    });
    
    // Verify no error message is displayed
    expect(screen.queryByText(/an unexpected error occurred/i)).not.toBeInTheDocument();
  });
  
  it('handles password reset error', async () => {
    mockResetPassword.mockResolvedValueOnce({ 
      error: { message: 'Email not found' } 
    });
    
    render(<ForgotPasswordForm />);
    
    // Fill in the email
    fireEvent.change(screen.getByLabelText(/email address/i), {
      target: { value: 'nonexistent@example.com' },
    });
    
    // Submit the form
    fireEvent.click(screen.getByRole('button', { name: /send reset link/i }));
    
    await waitFor(() => {
      // Verify error message is displayed
      expect(screen.getByText('Email not found')).toBeInTheDocument();
    });
    
    // Verify success message is not displayed
    expect(screen.queryByText(/check your email for a password reset link/i)).not.toBeInTheDocument();
  });
  
  it('handles unexpected errors', async () => {
    mockResetPassword.mockRejectedValueOnce(new Error('Network error'));
    
    render(<ForgotPasswordForm />);
    
    // Fill in the email
    fireEvent.change(screen.getByLabelText(/email address/i), {
      target: { value: 'test@example.com' },
    });
    
    // Submit the form
    fireEvent.click(screen.getByRole('button', { name: /send reset link/i }));
    
    await waitFor(() => {
      // Verify error message is displayed
      expect(screen.getByText(/an unexpected error occurred/i)).toBeInTheDocument();
    });
    
    // Verify success message is not displayed
    expect(screen.queryByText(/check your email for a password reset link/i)).not.toBeInTheDocument();
  });
  
  it('disables the submit button during form submission', async () => {
    // Create a delayed promise to simulate network request
    mockResetPassword.mockImplementationOnce(() => {
      return new Promise(resolve => {
        setTimeout(() => {
          resolve({ data: {}, error: null });
        }, 100);
      });
    });
    
    render(<ForgotPasswordForm />);
    
    // Fill in the email
    fireEvent.change(screen.getByLabelText(/email address/i), {
      target: { value: 'test@example.com' },
    });
    
    // Submit the form
    fireEvent.click(screen.getByRole('button', { name: /send reset link/i }));
    
    // Verify button is disabled and shows loading state
    expect(screen.getByRole('button', { name: /sending/i })).toBeDisabled();
    
    await waitFor(() => {
      // After completion, verify success message is displayed
      expect(screen.getByText(/check your email for a password reset link/i)).toBeInTheDocument();
    });
  });
});
