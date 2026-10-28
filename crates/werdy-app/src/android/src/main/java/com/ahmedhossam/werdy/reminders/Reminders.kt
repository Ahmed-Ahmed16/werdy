package com.ahmedhossam.werdy.reminders

import android.Manifest
import android.app.Activity
import android.app.AlarmManager
import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.net.Uri
import android.os.Build
import android.util.Log
import org.json.JSONArray
import org.json.JSONObject

// JNI callers get an error reply rather than a pending Java exception.
class Reminders(private val activity: Activity) {
    fun schedule(json: String): String = reply {
        AlarmStore.replace(activity.applicationContext, json)
        JSONObject().put("ok", true)
    }

    fun permission(): String = reply {
        JSONObject().put("allowed", AlarmStore.allowed(activity))
    }

    fun requestPermission(): String = reply {
        if (Build.VERSION.SDK_INT >= 33) {
            activity.runOnUiThread {
                try {
                    if (activity.checkSelfPermission(Manifest.permission.POST_NOTIFICATIONS) != PackageManager.PERMISSION_GRANTED) {
                        activity.requestPermissions(arrayOf(Manifest.permission.POST_NOTIFICATIONS), 2201)
                    }
                } catch (error: Exception) {
                    Log.w("werdy-reminders", "Notification permission request failed", error)
                }
            }
        }
        JSONObject().put("ok", true)
    }

    private fun reply(action: () -> JSONObject): String = try {
        action().toString()
    } catch (error: Exception) {
        JSONObject().put("error", error.message ?: error.javaClass.simpleName).toString()
    }
}

internal data class Alarm(val id: String, val epoch: Long, val title: String, val body: String)

internal object AlarmStore {
    private const val CHANNEL = "reminders"
    private const val PREFS = "werdy-reminders"
    private const val SAVED = "plan"

    private fun preferences(context: Context) = context.getSharedPreferences(PREFS, Context.MODE_PRIVATE)
    private fun manager(context: Context) = context.getSystemService(NotificationManager::class.java)
    private fun alarms(context: Context) = context.getSystemService(AlarmManager::class.java)

    private fun read(plan: JSONObject): List<Alarm> {
        val list = plan.getJSONArray("notifications")
        val seen = mutableSetOf<String>()
        return (0 until list.length()).map { index ->
            val entry = list.getJSONObject(index)
            val id = entry.getString("id")
            val epoch = entry.getLong("epoch_millis")
            require(id.isNotEmpty() && seen.add(id) && epoch >= 0) { "Invalid reminder identity or time" }
            Alarm(id, epoch, entry.getString("title"), entry.getString("body"))
        }
    }

    private fun saved(context: Context): JSONObject = JSONObject(
        preferences(context).getString(SAVED, null)
            ?: JSONObject().put("channel_name", CHANNEL).put("notifications", JSONArray()).toString()
    )

    // Intent data keeps two different ids distinct even if their hash codes collide.
    private fun intent(context: Context, alarm: Alarm) = Intent(context, ReminderReceiver::class.java)
        .setAction("com.ahmedhossam.werdy.REMINDER")
        .setData(Uri.parse("werdy-reminder:" + Uri.encode(alarm.id)))
        .putExtra("id", alarm.id)
        .putExtra("epoch_millis", alarm.epoch)

    private fun pending(context: Context, alarm: Alarm, flags: Int): PendingIntent? =
        PendingIntent.getBroadcast(context, alarm.id.hashCode(), intent(context, alarm), flags or PendingIntent.FLAG_IMMUTABLE)

    private fun arm(context: Context, alarm: Alarm) {
        if (alarm.epoch <= System.currentTimeMillis()) return
        val pending = pending(context, alarm, PendingIntent.FLAG_UPDATE_CURRENT)
            ?: return
        alarms(context).setAndAllowWhileIdle(AlarmManager.RTC_WAKEUP, alarm.epoch, pending)
    }

    @Synchronized
    fun replace(context: Context, json: String) {
        val plan = JSONObject(json)
        val next = read(plan)
        val channelName = plan.getString("channel_name")
        val previous = read(saved(context))
        for (alarm in previous) {
            pending(context, alarm, PendingIntent.FLAG_NO_CREATE)?.let {
                alarms(context).cancel(it)
                it.cancel()
            }
        }
        // Commit before arming: after process death, boot or the next save can recover the list.
        check(preferences(context).edit().putString(SAVED, plan.toString()).commit()) { "Could not save reminders" }
        channel(context, channelName)
        next.forEach { arm(context, it) }
    }

    @Synchronized
    fun restore(context: Context) {
        val plan = saved(context)
        val entries = read(plan)
        if (entries.isEmpty()) return
        channel(context, plan.getString("channel_name"))
        entries.forEach { arm(context, it) }
    }

    fun allowed(context: Context): Boolean {
        if (Build.VERSION.SDK_INT >= 33 && context.checkSelfPermission(Manifest.permission.POST_NOTIFICATIONS) != PackageManager.PERMISSION_GRANTED) return false
        if (!manager(context).areNotificationsEnabled()) return false
        return Build.VERSION.SDK_INT < 26 || manager(context).getNotificationChannel(CHANNEL)?.importance != NotificationManager.IMPORTANCE_NONE
    }

    private fun channel(context: Context, name: String) {
        if (Build.VERSION.SDK_INT >= 26) {
            manager(context).createNotificationChannel(NotificationChannel(CHANNEL, name, NotificationManager.IMPORTANCE_DEFAULT))
        }
    }

    @Synchronized
    fun deliver(context: Context, intent: Intent) {
        val plan = saved(context)
        val alarm = read(plan).find { it.id == intent.getStringExtra("id") && it.epoch == intent.getLongExtra("epoch_millis", -1) } ?: return
        if (!allowed(context)) return
        channel(context, plan.getString("channel_name"))
        val launch = context.packageManager.getLaunchIntentForPackage(context.packageName) ?: return
        launch.setData(Uri.parse("werdy-reminder-open:" + Uri.encode(alarm.id)))
        val content = PendingIntent.getActivity(context, alarm.id.hashCode(), launch, PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE)
        val builder = if (Build.VERSION.SDK_INT >= 26) Notification.Builder(context, CHANNEL) else Notification.Builder(context)
        val notification = builder.setSmallIcon(R.drawable.ic_reminder)
            .setContentTitle(alarm.title).setContentText(alarm.body)
            .setStyle(Notification.BigTextStyle().bigText(alarm.body))
            .setContentIntent(content).setAutoCancel(true).build()
        manager(context).notify(alarm.id, alarm.id.hashCode(), notification)
    }
}

class ReminderReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        try {
            AlarmStore.deliver(context.applicationContext, intent)
        } catch (error: Exception) {
            Log.w("werdy-reminders", "Reminder delivery failed", error)
        }
    }
}

class BootReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        if (intent.action != Intent.ACTION_BOOT_COMPLETED && intent.action != Intent.ACTION_MY_PACKAGE_REPLACED) return
        val pending = goAsync()
        Thread {
            try {
                AlarmStore.restore(context.applicationContext)
            } catch (error: Exception) {
                Log.w("werdy-reminders", "Reminder restore failed", error)
            } finally {
                pending.finish()
            }
        }.start()
    }
}

