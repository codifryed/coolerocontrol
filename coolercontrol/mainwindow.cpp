#include "mainwindow.h"

#include <QWebEngineView>
#include <QSystemTrayIcon>
#include <QMenu>
#include <QAction>
#include <QApplication>
#include <QSettings>
#include <QWebEngineNewWindowRequest>
#include <QStringBuilder> // for % operator
#include <QWebEngineSettings>
#include <QWizardPage>


MainWindow::MainWindow(QWidget *parent)
    : QMainWindow(parent)
      , view(new QWebEngineView(this))
      , profile(new QWebEngineProfile("coolercontrol", view))
      , page(new QWebEnginePage(profile))
      , channel(new QWebChannel(page))
      , ipc(new IPC) {
    // SETUP
    ////////////////////////////////////////////////////////////////////////////////////////////////
    setCentralWidget(view);
    profile->settings()->setAttribute(QWebEngineSettings::Accelerated2dCanvasEnabled, true);
    profile->settings()->setAttribute(QWebEngineSettings::ScreenCaptureEnabled, false);
    profile->settings()->setAttribute(QWebEngineSettings::PluginsEnabled, false);
    profile->settings()->setAttribute(QWebEngineSettings::PdfViewerEnabled, false);
    // local storage: ~/.local/share/{APP_NAME}
    profile->settings()->setAttribute(QWebEngineSettings::LocalStorageEnabled, true);
    profile->setPersistentCookiesPolicy(QWebEngineProfile::PersistentCookiesPolicy::ForcePersistentCookies);
    channel->registerObject("ipc", ipc);
    page->setWebChannel(channel);
    // This allows external links in our app to be opened by the external browser:
    connect(page, &QWebEnginePage::newWindowRequested, [](QWebEngineNewWindowRequest const &request) {
        QDesktopServices::openUrl(request.requestedUrl());
    });
    view->setPage(page);

    // SYSTEM TRAY:
    ////////////////////////////////////////////////////////////////////////////////////////////////
    closing = false;
    const auto ccHeader = new QAction(QIcon(":/icons/icon.png"), tr("CoolerControl"), this);
    // todo: we could enable this for a 'prettier' sys tray header - maybe use the hide/show logic on Triggered
    ccHeader->setDisabled(true);
    showAction = new QAction(QIcon::fromTheme("window-close", QIcon()), tr("&Hide"), this);
    connect(showAction, &QAction::triggered, [this]() {
        if (isVisible()) {
            hide();
            showAction->setText(tr("&Show"));
            showAction->setIcon(QIcon::fromTheme("window-new", QIcon()));
        } else {
            show();
            activateWindow();
            showAction->setText(tr("&Hide"));
            showAction->setIcon(QIcon::fromTheme("window-close", QIcon()));
        }
    });

    addressAction = new QAction(QIcon::fromTheme("address-book-new", QIcon()), tr("&Daemon Address"), this);
    connect(addressAction, &QAction::triggered, [this]() {
        displayAddressWizard();
    });

    quitAction = new QAction(QIcon::fromTheme("application-exit", QIcon()), tr("&Quit"), this);
    connect(quitAction, &QAction::triggered, [this]() {
        closing = true;
        // This closes the window, but doesn't quit the application
        close();
    });
    trayIconMenu = new QMenu(this);
    trayIconMenu->addAction(ccHeader);
    trayIconMenu->addSeparator();
    trayIconMenu->addAction(showAction);
    trayIconMenu->addAction(addressAction);
    trayIconMenu->addSeparator();
    trayIconMenu->addAction(quitAction);

    sysTrayIcon = new QSystemTrayIcon(this);
    sysTrayIcon->setContextMenu(trayIconMenu);
    sysTrayIcon->setIcon(QIcon(":/icons/icon.ico"));
    sysTrayIcon->setToolTip("CoolerControl");
    sysTrayIcon->show();

    // left click:
    connect(sysTrayIcon, &QSystemTrayIcon::activated, [this](auto reason) {
        if (reason == QSystemTrayIcon::Trigger) {
            if (isVisible()) {
                hide();
                showAction->setText(tr("&Show"));
            } else {
                show();
                activateWindow();
                showAction->setText(tr("&Hide"));
            }
        }
    });

    // LOAD UI:
    ////////////////////////////////////////////////////////////////////////////////////////////////
    view->load(getDaemonUrl());
    connect(view, &QWebEngineView::loadFinished, [this](const bool pageLoadedSuccessfully) {
        if (!pageLoadedSuccessfully) {
            displayAddressWizard();
        }
    });

    // todo: zoom adjustement (probably with webchannel)
    // view->setZoomFactor()
    // todo: we can probably change the log download blob/link in the UI to point to an external link to see the raw text api endpoint

    // todo: check for existing running CC application? (there must be some standard for Qt???)
}

void MainWindow::closeEvent(QCloseEvent *event) {
    event->accept();
    QApplication::quit();
    // todo: logic for CloseToTray setting:
    // if (closing) {
    //     event->accept();
    //     deleteLater();
    //     QApplication::quit();
    // } else {
    //     this->hide();
    //     event->ignore();
    // }
}

QUrl MainWindow::getDaemonUrl() {
    const QSettings settings;
    const auto host = settings.value(SETTING_DAEMON_ADDRESS, DEFAULT_DAEMON_ADDRESS.data()).toString();
    const auto port = settings.value(SETTING_DAEMON_PORT, DEFAULT_DAEMON_PORT).toInt();
    const auto sslEnabled = settings.value(SETTING_DAEMON_SSL_ENABLED, DEFAULT_DAEMON_SSL_ENABLED).toBool();
    const auto prefix = sslEnabled ? tr("https://") : tr("http://");
    return QUrl(prefix % host % tr(":") % QString::number(port));
}

void MainWindow::displayAddressWizard() {
    if (wizard == nullptr) {
        wizard = new QWizard;
        wizard->setWindowTitle("Daemon Connection Error");
        wizard->setButtonText(QWizard::WizardButton::FinishButton, "&Apply");
        wizard->setButtonText(QWizard::WizardButton::CancelButton, "&Quit");
        wizard->setButtonText(QWizard::CustomButton1, "&Reset");
        wizard->setOption(QWizard::HaveCustomButton1, true);
        wizard->addPage(new IntroPage);
        auto addressPage = new AddressPage;
        wizard->addPage(addressPage);
        connect(wizard, &QWizard::customButtonClicked, [this, addressPage]() {
            addressPage->resetAddressInputValues();
        });
    }
    if (wizard->isVisible()) {
        return;
    }
    const auto result = wizard->exec();
    if (result == 0) {
        QApplication::quit();
    } else {
        QSettings settings;
        settings.setValue(SETTING_DAEMON_ADDRESS, wizard->field("address").toString());
        settings.setValue(SETTING_DAEMON_PORT, wizard->field("port").toInt());
        settings.setValue(SETTING_DAEMON_SSL_ENABLED, wizard->field("ssl").toBool());
        view->load(getDaemonUrl());
    }
}
