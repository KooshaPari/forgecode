"use client";

import React, { createContext, useContext, useEffect, useState } from "react";
import { Session, User, AuthError } from "@supabase/supabase-js";
import { supabase } from "@/lib/supabase";

type AuthContextType = {
	session: Session | null;
	user: User | null;
	loading: boolean;
	signIn: (
		email: string,
		password: string
	) => Promise<{
		error: AuthError | null;
		data: {
			user: User | null;
			session: Session | null;
		} | null;
	}>;
	signUp: (
		email: string,
		password: string
	) => Promise<{
		error: AuthError | null;
		data: {
			user: User | null;
			session: Session | null;
		} | null;
	}>;
	signOut: () => Promise<{
		error: AuthError | null;
	}>;
	resetPassword: (email: string) => Promise<{
		error: AuthError | null;
		data: {} | null;
	}>;
};

const AuthContext = createContext<AuthContextType | undefined>(undefined);

export function AuthProvider({ children }: { children: React.ReactNode }) {
	const [session, setSession] = useState<Session | null>(null);
	const [user, setUser] = useState<User | null>(null);
	const [loading, setLoading] = useState(true);

	useEffect(() => {
		// Get initial session
		supabase.auth.getSession().then(({ data: { session } }) => {
			setSession(session);
			setUser(session?.user ?? null);
			setLoading(false);
		});

		// Listen for auth changes
		const {
			data: { subscription },
		} = supabase.auth.onAuthStateChange((_event, session) => {
			setSession(session);
			setUser(session?.user ?? null);
			setLoading(false);
		});

		return () => {
			subscription.unsubscribe();
		};
	}, []);

	const signIn = async (email: string, password: string) => {
		const { data, error } = await supabase.auth.signInWithPassword({
			email,
			password,
		});
		return { data, error };
	};

	const signUp = async (email: string, password: string) => {
		const { data, error } = await supabase.auth.signUp({
			email,
			password,
		});
		return { data, error };
	};

	const signOut = async () => {
		const { error } = await supabase.auth.signOut();
		return { error };
	};

	const resetPassword = async (email: string) => {
		const { data, error } = await supabase.auth.resetPasswordForEmail(email, {
			redirectTo: `${window.location.origin}/reset-password`,
		});
		return { data, error };
	};

	const value = {
		session,
		user,
		loading,
		signIn,
		signUp,
		signOut,
		resetPassword,
	};

	return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>;
}

export function useAuth() {
	const context = useContext(AuthContext);
	if (context === undefined) {
		throw new Error("useAuth must be used within an AuthProvider");
	}
	return context;
}
