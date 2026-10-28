package com.ahmedhossam.werdy.signin

import android.app.Activity
import androidx.credentials.CredentialManager
import androidx.credentials.CustomCredential
import androidx.credentials.GetCredentialRequest
import androidx.credentials.Credential
import androidx.credentials.exceptions.NoCredentialException
import com.google.android.libraries.identity.googleid.GetGoogleIdOption
import com.google.android.libraries.identity.googleid.GetSignInWithGoogleOption
import com.google.android.libraries.identity.googleid.GoogleIdTokenCredential
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.TimeoutCancellationException
import kotlinx.coroutines.launch
import kotlinx.coroutines.withTimeout
import org.json.JSONObject

class GoogleSignin(private val activity: Activity) {
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Main)
    private var request: Job? = null
    @Volatile private var reply = """{"state":"pending"}"""

    @Synchronized
    fun start(webClientId: String, nonce: String): String {
        return try {
            if (request != null) return reply
            reply = """{"state":"pending"}"""
            request = scope.launch { fetchToken(webClientId, nonce) }
            """{"state":"pending"}"""
        } catch (error: Exception) {
            reply = failure(error.javaClass.simpleName, error.message ?: "Could not start Google sign in")
            reply
        }
    }

    fun poll(): String = reply

    @Synchronized
    fun cancel(): String {
        return try {
            request?.cancel()
            """{"state":"pending"}"""
        } catch (error: Exception) {
            reply = failure(error.javaClass.simpleName, error.message ?: "Could not cancel Google sign in")
            reply
        }
    }

    private suspend fun fetchToken(webClientId: String, nonce: String) {
        try {
            val token = withTimeout(120_000) { googleToken(webClientId, nonce) }
            reply = JSONObject().put("state", "ok").put("idToken", token).toString()
        } catch (error: TimeoutCancellationException) {
            reply = failure("Timeout", "Google sign in exceeded 120 seconds")
        } catch (error: CancellationException) {
            reply = failure("Cancelled", "Google sign in cancelled")
        } catch (error: Exception) {
            reply = failure(error.javaClass.simpleName, error.message ?: "Google sign in failed")
        }
    }

    private suspend fun googleToken(webClientId: String, nonce: String): String {
        val manager = CredentialManager.create(activity)
        val option = GetGoogleIdOption.Builder()
            .setServerClientId(webClientId)
            .setFilterByAuthorizedAccounts(false)
            .setNonce(nonce)
            .build()
        val credential = try {
            manager.getCredential(activity, GetCredentialRequest.Builder().addCredentialOption(option).build()).credential
        } catch (error: NoCredentialException) {
            val fallback = GetSignInWithGoogleOption.Builder(webClientId).setNonce(nonce).build()
            manager.getCredential(activity, GetCredentialRequest.Builder().addCredentialOption(fallback).build()).credential
        }
        return idToken(credential)
    }

    private fun idToken(credential: Credential): String {
        require(credential is CustomCredential && credential.type == GoogleIdTokenCredential.TYPE_GOOGLE_ID_TOKEN_CREDENTIAL) {
            "Unexpected credential type"
        }
        return GoogleIdTokenCredential.createFrom(credential.data).idToken
    }

    private fun failure(kind: String, message: String): String =
        JSONObject().put("state", "error").put("kind", kind).put("message", message).toString()
}
