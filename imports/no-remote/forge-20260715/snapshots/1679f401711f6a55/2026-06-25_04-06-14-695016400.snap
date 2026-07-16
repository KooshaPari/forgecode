import React from 'react';
import { render, screen, fireEvent } from '@testing-library/react';
import Navigation from '@/components/layout/Navigation';
import { useAuth } from '@/contexts/AuthContext';
import { usePathname } from 'next/navigation';

// Mock the useAuth hook
jest.mock('@/contexts/AuthContext', () => ({
  useAuth: jest.fn(),
}));

// Mock the usePathname hook
jest.mock('next/navigation', () => ({
  usePathname: jest.fn(),
}));

describe('Navigation', () => {
  const mockSignOut = jest.fn();
  
  beforeEach(() => {
    jest.clearAllMocks();
    
    // Setup mocks
    (useAuth as jest.Mock).mockReturnValue({
      user: { email: 'test@example.com' },
      signOut: mockSignOut,
    });
    
    (usePathname as jest.Mock).mockReturnValue('/dashboard');
  });
  
  it('renders the navigation with user logged in', () => {
    render(<Navigation />);
    
    // Check logo is present
    expect(screen.getByText('LocalBase')).toBeInTheDocument();
    
    // Check navigation links
    expect(screen.getByText('Dashboard')).toBeInTheDocument();
    expect(screen.getByText('Jobs')).toBeInTheDocument();
    expect(screen.getByText('API Keys')).toBeInTheDocument();
    
    // Check user menu
    expect(screen.getByText('test@example.com')).toBeInTheDocument();
  });
  
  it('highlights the active navigation item', () => {
    (usePathname as jest.Mock).mockReturnValue('/jobs');
    
    render(<Navigation />);
    
    // Dashboard link should not be highlighted
    const dashboardLink = screen.getByText('Dashboard').closest('a');
    expect(dashboardLink).not.toHaveClass('bg-indigo-700');
    
    // Jobs link should be highlighted
    const jobsLink = screen.getByText('Jobs').closest('a');
    expect(jobsLink).toHaveClass('bg-indigo-700');
  });
  
  it('opens and closes the mobile menu', () => {
    render(<Navigation />);
    
    // Mobile menu should be closed initially
    expect(screen.queryByRole('menu')).not.toBeInTheDocument();
    
    // Open mobile menu
    fireEvent.click(screen.getByLabelText('Open menu'));
    
    // Mobile menu should be open
    const mobileMenu = screen.getByRole('menu');
    expect(mobileMenu).toBeInTheDocument();
    
    // Close mobile menu
    fireEvent.click(screen.getByLabelText('Close menu'));
    
    // Mobile menu should be closed
    expect(screen.queryByRole('menu')).not.toBeInTheDocument();
  });
  
  it('opens and closes the user dropdown', () => {
    render(<Navigation />);
    
    // User dropdown should be closed initially
    expect(screen.queryByText('Profile')).not.toBeInTheDocument();
    expect(screen.queryByText('Sign out')).not.toBeInTheDocument();
    
    // Open user dropdown
    fireEvent.click(screen.getByText('test@example.com'));
    
    // User dropdown should be open
    expect(screen.getByText('Profile')).toBeInTheDocument();
    expect(screen.getByText('Sign out')).toBeInTheDocument();
    
    // Close user dropdown by clicking outside
    fireEvent.click(document.body);
    
    // User dropdown should be closed
    expect(screen.queryByText('Profile')).not.toBeInTheDocument();
    expect(screen.queryByText('Sign out')).not.toBeInTheDocument();
  });
  
  it('calls signOut when sign out button is clicked', () => {
    render(<Navigation />);
    
    // Open user dropdown
    fireEvent.click(screen.getByText('test@example.com'));
    
    // Click sign out button
    fireEvent.click(screen.getByText('Sign out'));
    
    // Verify signOut was called
    expect(mockSignOut).toHaveBeenCalled();
  });
  
  it('renders without user when not logged in', () => {
    // Mock user as null (not logged in)
    (useAuth as jest.Mock).mockReturnValue({
      user: null,
      signOut: mockSignOut,
    });
    
    render(<Navigation />);
    
    // Check logo is present
    expect(screen.getByText('LocalBase')).toBeInTheDocument();
    
    // User email should not be present
    expect(screen.queryByText('test@example.com')).not.toBeInTheDocument();
    
    // Sign in and Sign up links should be present
    expect(screen.getByText('Sign in')).toBeInTheDocument();
    expect(screen.getByText('Sign up')).toBeInTheDocument();
  });
});
